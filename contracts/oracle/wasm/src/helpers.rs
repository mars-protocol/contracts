use std::{
    cmp::{min, Ordering},
    str::FromStr,
};

use astroport::{
    asset::{Asset, AssetInfo, PairInfo},
    factory::PairType,
    pair::{ConfigResponse, CumulativePricesResponse, PoolResponse, QueryMsg as PairQueryMsg},
    pair_concentrated::QueryMsg as ConcentratedPairQueryMsg,
};
use cosmwasm_std::{
    ensure_eq, to_json_binary, Addr, Decimal, Decimal256, Deps, Env, QuerierWrapper, QueryRequest,
    StdResult, Uint128, WasmQuery,
};
use cw_storage_plus::Map;
use mars_oracle_base::{ContractError, ContractResult, PriceSourceChecked};
use mars_types::oracle::{ActionKind, AstroportTwapSnapshot, Config};

use crate::WasmPriceSourceChecked;

/// Queries the pair contract for the pair info.
pub fn query_astroport_pair_info(
    querier: &QuerierWrapper,
    pair_contract: impl Into<String>,
) -> StdResult<PairInfo> {
    querier.query_wasm_smart(pair_contract, &PairQueryMsg::Pair {})
}

/// Queries the pair contract for the pool info.
pub fn query_astroport_pool(
    querier: &QuerierWrapper,
    pair_contract: impl Into<String>,
) -> StdResult<PoolResponse> {
    querier.query_wasm_smart(pair_contract, &PairQueryMsg::Pool {})
}

pub fn query_astroport_config(
    querier: &QuerierWrapper,
    pair_contract: impl Into<String>,
) -> StdResult<ConfigResponse> {
    querier.query_wasm_smart(pair_contract, &PairQueryMsg::Config {})
}

pub fn query_astroport_curve_invariant(
    querier: &QuerierWrapper,
    pair_contract: impl Into<String>,
) -> StdResult<Decimal256> {
    querier.query_wasm_smart(pair_contract, &ConcentratedPairQueryMsg::ComputeD {})
}

/// Helper function to create an Astroport native token AssetInfo.
pub fn astro_native_info(denom: &str) -> AssetInfo {
    AssetInfo::NativeToken {
        denom: denom.to_string(),
    }
}

/// Helper function to create an Astroport native Asset.
pub fn astro_native_asset(denom: impl Into<String>, amount: impl Into<Uint128>) -> Asset {
    Asset {
        info: astro_native_info(&denom.into()),
        amount: amount.into(),
    }
}

pub fn validate_astroport_pair_price_source(
    deps: &Deps,
    pair_address: &Addr,
    denom: &str,
    base_denom: &str,
    price_sources: &Map<&str, WasmPriceSourceChecked>,
) -> ContractResult<()> {
    // Get the denoms of the pair
    let pair_info = query_astroport_pair_info(&deps.querier, pair_address)?;
    let pair_denoms = get_astroport_pair_denoms(&pair_info)?;

    // Get the other denom of the pair. This also checks that the pair contains the first denom
    // and that the pair only has two assets.
    let other_pair_denom = get_other_astroport_pair_denom(&pair_denoms, denom)?;

    // If the pair does not contain the base denom, a price source for the other denom of the pair
    // must exist.
    if !pair_denoms.contains(&base_denom.to_string())
        && !price_sources.has(deps.storage, &other_pair_denom)
    {
        return Err(ContractError::InvalidPriceSource {
            reason: format!(
                "pair does not contain base denom and no price source is configured for the other denom {}",
                other_pair_denom
            ),
        });
    }

    Ok(())
}

/// Gets the native denoms from an Astroport pair. Fails if the pair contains a CW20 token.
pub fn get_astroport_pair_denoms(pair_info: &PairInfo) -> ContractResult<Vec<String>> {
    pair_info
        .asset_infos
        .iter()
        .map(|a| match a {
            AssetInfo::Token {
                contract_addr,
            } => Err(ContractError::InvalidPriceSource {
                reason: format!("pair contains cw20 token: {}", contract_addr),
            }),
            AssetInfo::NativeToken {
                denom,
            } => Ok(denom.clone()),
        })
        .collect()
}

/// Gets the other native denom of an Astroport pair. Fails if the pair contains more than two assets or
/// if the pair does not contain the specified denom.
pub fn get_other_astroport_pair_denom(
    pair_denoms: &[String],
    denom: &str,
) -> ContractResult<String> {
    if pair_denoms.len() != 2 {
        return Err(ContractError::InvalidPriceSource {
            reason: format!("pair contains more than two assets: {:?}", pair_denoms),
        });
    }
    if !pair_denoms.contains(&denom.to_string()) {
        return Err(ContractError::InvalidPriceSource {
            reason: format!("pair does not contain denom {}", denom),
        });
    }
    if pair_denoms[0] == denom {
        Ok(pair_denoms[1].clone())
    } else {
        Ok(pair_denoms[0].clone())
    }
}

/// Queries the Astroport factory contract for the token precision of the specified denom.
pub fn query_token_precision(
    querier: &QuerierWrapper,
    astroport_factory: &Addr,
    denom: &str,
) -> ContractResult<u8> {
    Ok(astroport::querier::query_token_precision(
        querier,
        &AssetInfo::NativeToken {
            denom: denom.to_string(),
        },
        astroport_factory,
    )?)
}

/// Queries the pair contract for the cumulate price of the specified denom denominated in the other
/// asset of the pair.
pub fn query_astroport_cumulative_price(
    querier: &QuerierWrapper,
    pair_address: &Addr,
    denom: &str,
) -> Result<Uint128, ContractError> {
    let response: CumulativePricesResponse =
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: pair_address.to_string(),
            msg: to_json_binary(&PairQueryMsg::CumulativePrices {})?,
        }))?;

    let (_, _, price) =
        response.cumulative_prices.iter().find(|(from, _, _)| from.to_string() == denom).ok_or(
            // This error should not happen, but lets return it instead of unwrapping anyway
            ContractError::InvalidPriceSource {
                reason: format!("Cumulative price not found for asset {}", denom),
            },
        )?;

    Ok(*price)
}

/// Calculate how much the period between two TWAP snapshots deviates from the desired window size
pub fn period_diff(
    snapshot1: &AstroportTwapSnapshot,
    snapshot2: &AstroportTwapSnapshot,
    window_size: u64,
) -> u64 {
    snapshot1.timestamp.abs_diff(snapshot2.timestamp).abs_diff(window_size)
}

/// Check if the pair contains the base denom. If not then normalize the price to the base denom
/// by querying the price source for the other denom of the pair. If the pair contains the base
/// denom then return the price as is.
pub fn normalize_price(
    deps: &Deps,
    env: &Env,
    config: &Config,
    price_sources: &Map<&str, WasmPriceSourceChecked>,
    pair_info: &PairInfo,
    denom: &str,
    price: Decimal,
    kind: ActionKind,
) -> ContractResult<Decimal> {
    let pair_denoms = get_astroport_pair_denoms(pair_info)?;

    if pair_denoms.contains(&config.base_denom) {
        Ok(price)
    } else {
        let other_pair_denom = get_other_astroport_pair_denom(&pair_denoms, denom)?;

        let other_price_source = price_sources.load(deps.storage, &other_pair_denom)?;
        let other_price = other_price_source.query_price(
            deps,
            env,
            &other_pair_denom,
            config,
            price_sources,
            kind,
        )?;

        Ok(price.checked_mul(other_price)?)
    }
}

/// Adjusts the precision of `value` from `current_precision` to `new_precision`. Copied from
/// https://github.com/astroport-fi/astroport-core/blob/v2.8.0/contracts/pair_stable/src/utils.rs#L139
/// because it is not public.
pub fn adjust_precision(
    value: Uint128,
    current_precision: u8,
    new_precision: u8,
) -> ContractResult<Uint128> {
    Ok(match current_precision.cmp(&new_precision) {
        Ordering::Equal => value,
        Ordering::Less => value
            .checked_mul(Uint128::new(10_u128.pow((new_precision - current_precision) as u32)))?,
        Ordering::Greater => value
            .checked_div(Uint128::new(10_u128.pow((current_precision - new_precision) as u32)))?,
    })
}

pub fn validate_astroport_lp_pool_for_type(
    deps: &Deps,
    pair_address: &Addr,
    price_sources: &Map<&str, WasmPriceSourceChecked>,
    pair_type: PairType,
) -> ContractResult<()> {
    let pair_info = query_astroport_pair_info(&deps.querier, pair_address)?;
    ensure_eq!(
        pair_info.pair_type,
        pair_type,
        ContractError::InvalidPriceSource {
            reason: format!(
                "expecting pair {} to be {} pool; found {}",
                pair_address, pair_type, pair_info.pair_type
            ),
        }
    );

    let pair_denoms = get_astroport_pair_denoms(&pair_info)?;
    if pair_denoms.len() != 2 {
        return Err(ContractError::InvalidPriceSource {
            reason: format!(
                "expecting pair {} to contain exactly two coins; found {}",
                pair_address,
                pair_denoms.len()
            ),
        });
    }

    for denom in pair_denoms.iter() {
        if !price_sources.has(deps.storage, denom) {
            return Err(ContractError::InvalidPriceSource {
                reason: format!("missing price source for {}", denom),
            });
        }
    }

    Ok(())
}

pub fn compute_pcl_lp_price(
    coin0_decimals: u8,
    coin1_decimals: u8,
    coin0_price: &Decimal, // This is the price per whole unit
    coin1_price: &Decimal,
    coin0_amount: Uint128,
    coin1_amount: Uint128,
    total_shares: Uint128,
    price_scale: Decimal,
    curve_invariant: Decimal256,
) -> ContractResult<Decimal> {
    let lp_price_model = compute_pcl_lp_price_model(
        coin0_price,
        coin1_price,
        total_shares,
        price_scale,
        curve_invariant,
    )?;

    let lp_price_real = compute_pcl_lp_price_real(
        coin0_decimals,
        coin1_decimals,
        coin0_amount,
        coin1_amount,
        coin0_price,
        coin1_price,
        total_shares,
    )?;

    let pcl_lp_price = min(lp_price_model, lp_price_real);

    Ok(pcl_lp_price)
}

pub fn compute_pcl_lp_price_model(
    coin0_price: &Decimal,
    coin1_price: &Decimal,
    total_shares: Uint128,
    price_scale: Decimal,
    curve_invariant: Decimal256,
) -> ContractResult<Decimal> {
    // xcp represents the virtual value of the pool
    // xcp = curve_invariant / (2 * sqrt(price_scale))
    let xcp = curve_invariant.checked_div(
        Decimal256::from(price_scale).sqrt().checked_mul(Decimal256::from_str("2")?)?,
    )?;

    // Virtual price represents the theoretic price of one share. This virtual price is used as input
    // for the Curve V2 model to determine the modelled lp price.
    // virtual_price = xcp / total_shares
    let virtual_price = xcp.checked_div(Decimal256::from_ratio(total_shares, 1u128))?;

    // LP price according to the model
    // lp_price_model = 2 * virtual_price * sqrt(price)
    let lp_price_model_256 = Decimal256::from_str("2")?
        .checked_mul(virtual_price)?
        .checked_mul(Decimal256::from(coin0_price.checked_mul(*coin1_price)?.sqrt()))?;
    let lp_price_model = Decimal::try_from(lp_price_model_256)?;

    Ok(lp_price_model)
}

pub fn compute_pcl_lp_price_real(
    coin0_decimals: u8,
    coin1_decimals: u8,
    coin0_amount: Uint128,
    coin1_amount: Uint128,
    coin0_price: &Decimal,
    coin1_price: &Decimal,
    total_shares: Uint128,
) -> ContractResult<Decimal> {
    // As the oracle returns prices for whole units, we need to adjust the amount to the correct
    // precision before calculating the real TVL.
    let coin0_amount_adjusted =
        Decimal::from_ratio(coin0_amount, 10_u128.pow((coin0_decimals) as u32));
    let coin1_amount_adjusted =
        Decimal::from_ratio(coin1_amount, 10_u128.pow((coin1_decimals) as u32));

    // Need to use Decimal256 because coin0_amount * price + coin1_amount may overflow the Decimal limit
    // E.g. 1000 BTC + 21000 ETH in a pool, with a price of 65000 and 3000:
    // price = 650 / 0.0000000000003 = 1_267_000_000_000_000
    // 1_000_000_000_000 * 1_267_000_000_000_000 + 21_000_000_000_000_000_000_000 > Decimal::MAX
    let tvl_real = Decimal256::from(coin0_amount_adjusted) * Decimal256::from(*coin0_price)
        + Decimal256::from(coin1_amount_adjusted) * Decimal256::from(*coin1_price);

    let lp_price_real_256 = tvl_real.checked_div(Decimal256::from_ratio(total_shares, 1u128))?;
    let lp_price_real = Decimal::try_from(lp_price_real_256)?;

    Ok(lp_price_real)
}
