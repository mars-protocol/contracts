use astroport::{
    asset::{Asset, AssetInfo, PairInfo},
    pair::{CumulativePricesResponse, QueryMsg as PairQueryMsg},
};
use cosmwasm_std::{
    to_binary, Addr, Decimal, Deps, Env, QuerierWrapper, QueryRequest, StdResult, Uint128,
    WasmQuery,
};
use cw_storage_plus::Map;
use mars_oracle_base::{ContractError, ContractResult, PriceSourceChecked};
use mars_red_bank_types::oracle::{AstroportTwapSnapshot, Config};

use crate::WasmPriceSourceChecked;

/// Queries the pair contract for the pair info.
pub fn query_astroport_pair_info(
    querier: &QuerierWrapper,
    pair_contract: impl Into<String>,
) -> StdResult<PairInfo> {
    querier.query_wasm_smart(pair_contract, &PairQueryMsg::Pair {})
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
    let mut pair_denoms = get_astroport_pair_denoms(deps, pair_address)?;

    // Pair must contain two assets
    if pair_denoms.len() != 2 {
        return Err(ContractError::InvalidPriceSource {
            reason: format!("pair contains more than two assets: {:?}", pair_denoms),
        });
    }

    // Pair must contain the denom
    if !pair_denoms.contains(&denom.to_string()) {
        return Err(ContractError::InvalidPriceSource {
            reason: format!("pair does not contain denom {}", denom),
        });
    }

    // Get the other denom of the pair. This works because we asserted above that the pair contains
    // exactly two assets.
    pair_denoms.retain(|d| d != denom);
    let other_pair_denom = pair_denoms.first().unwrap();

    // If the pair does not contain the base denom, a price source for the other denom of the pair
    // must exist.
    if !pair_denoms.contains(&base_denom.to_string())
        && !price_sources.has(deps.storage, other_pair_denom)
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

pub fn get_astroport_pair_denoms(deps: &Deps, pair_address: &Addr) -> ContractResult<Vec<String>> {
    let pair_info = query_astroport_pair_info(&deps.querier, pair_address)?;
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
            msg: to_binary(&PairQueryMsg::CumulativePrices {})?,
        }))?;

    let (_, _, price) =
        response.cumulative_prices.iter().find(|(d, _, _)| d.to_string() == denom).ok_or(
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
    pair_address: &Addr,
    denom: &str,
    price: Decimal,
) -> ContractResult<Decimal> {
    let mut pair_denoms = get_astroport_pair_denoms(deps, pair_address)?;

    if pair_denoms.contains(&config.base_denom) {
        Ok(price)
    } else {
        // In validate we assert that the pair contains the denom, and that it contains only
        // two denoms.
        pair_denoms.retain(|d| d != denom);
        let other_pair_denom = pair_denoms.first().unwrap();

        let other_price_source = price_sources.load(deps.storage, other_pair_denom)?;
        let other_price =
            other_price_source.query_price(deps, env, other_pair_denom, config, price_sources)?;

        Ok(price.checked_mul(other_price)?)
    }
}

#[cfg(test)]
mod tests {}
