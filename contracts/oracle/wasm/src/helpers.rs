use std::collections::HashSet;

use astroport::{
    asset::{Asset, AssetInfo, PairInfo},
    pair::{CumulativePricesResponse, QueryMsg as PairQueryMsg},
};
use cosmwasm_std::{
    to_binary, Addr, Decimal, Deps, Env, QuerierWrapper, QueryRequest, StdResult, Uint128,
    WasmQuery,
};
use cw_storage_plus::Map;
use mars_oracle::AstroportTwapSnapshot;
use mars_oracle_base::{ContractError, ContractResult, PriceSourceChecked};

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

/// Validates the route assets of an Astroport price source. Used for both TWAP and spot price sources.
pub fn validate_route_assets(
    deps: &Deps,
    denom: &str,
    base_denom: &str,
    price_sources: &Map<&str, WasmPriceSourceChecked>,
    pair_address: &str,
    route_assets: &[String],
) -> ContractResult<()> {
    // For all route assets, there must be a price source available
    for asset in route_assets {
        if !price_sources.has(deps.storage, asset) {
            Err(ContractError::InvalidPriceSource {
                reason: format!("No price source found for asset {}", asset),
            })?;
        }
    }

    // Route assets should be unique
    let mut route_assets_set = HashSet::new();
    for asset in route_assets {
        if !route_assets_set.insert(asset) {
            Err(ContractError::InvalidPriceSource {
                reason: format!("Duplicate route asset {}", asset),
            })?;
        }
    }

    // Route assets can not contain the price source's denom
    if route_assets.contains(&denom.to_string()) {
        Err(ContractError::InvalidPriceSource {
            reason: format!("Route assets contain the price source denom {}", denom),
        })?;
    }

    let pair_info = query_astroport_pair_info(&deps.querier, pair_address)?;

    if route_assets.is_empty() {
        // If there are no route assets, then the pair must contain the denom and base denom.
        assert_astroport_pair_contains_denoms(&pair_info, &[denom, base_denom])?;
    } else {
        // If there are route assets, the pair must contain the denom and the first route asset.
        // The rest should already be validated in the same way because we checked above that a
        // price source exists for each of them, and the corresponding validation would have been
        // done when they were added.
        assert_astroport_pair_contains_denoms(&pair_info, &[denom, &route_assets[0]])?;
    }

    Ok(())
}

/// Asserts that the pair contains exactly the specified denoms.
pub fn assert_astroport_pair_contains_denoms(
    pair_info: &PairInfo,
    denoms: &[&str],
) -> ContractResult<()> {
    let pair_denoms: HashSet<_> = pair_info.asset_infos.iter().map(|a| a.to_string()).collect();
    let denoms: HashSet<_> = denoms.iter().map(|s| s.to_string()).collect();

    if pair_denoms != denoms {
        return Err(ContractError::InvalidPriceSource {
            reason: format!(
                "pair {} does not contain the denoms {:?}",
                pair_info.contract_addr, denoms
            ),
        });
    }
    Ok(())
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

/// Add the prices of the route assets to the supplied price to get a price denominated in the base
/// asset.
pub fn add_route_prices(
    deps: &Deps,
    env: &Env,
    base_denom: &str,
    price_sources: &Map<&str, WasmPriceSourceChecked>,
    route_assets: &[String],
    price: &Decimal,
) -> ContractResult<Decimal> {
    let mut price = *price;
    for denom in route_assets {
        let price_source = price_sources.load(deps.storage, denom)?;
        let route_price = price_source.query_price(deps, env, denom, base_denom, price_sources)?;
        price *= route_price;
    }
    Ok(price)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Addr;

    use super::*;

    #[test]
    fn test_assert_astroport_pair_contains_denoms() {
        let pair_info = PairInfo {
            contract_addr: Addr::unchecked("pair_contract"),
            asset_infos: vec![
                astro_native_info("uusd"),
                astro_native_info("uatom"),
                astro_native_info("uluna"),
            ],
            liquidity_token: Addr::unchecked("liquidity_token"),
            pair_type: astroport::factory::PairType::Xyk {},
        };

        assert_astroport_pair_contains_denoms(&pair_info, &["uusd", "uatom", "uluna"]).unwrap();
        assert_astroport_pair_contains_denoms(&pair_info, &["uusd", "uluna", "uatom"]).unwrap();
        assert_astroport_pair_contains_denoms(&pair_info, &["uatom", "uusd", "uluna"]).unwrap();
        assert_astroport_pair_contains_denoms(&pair_info, &["uusd"]).unwrap_err();
        assert_astroport_pair_contains_denoms(&pair_info, &["uatom"]).unwrap_err();
        assert_astroport_pair_contains_denoms(&pair_info, &["uluna"]).unwrap_err();

        assert_astroport_pair_contains_denoms(&pair_info, &["uusd", "uatom", "uluna", "ukrw"])
            .unwrap_err();
        assert_astroport_pair_contains_denoms(&pair_info, &["uusd", "uatom", "ukrw"]).unwrap_err();
        assert_astroport_pair_contains_denoms(&pair_info, &["uusd", "ukrw"]).unwrap_err();
        assert_astroport_pair_contains_denoms(&pair_info, &["uatom", "ukrw"]).unwrap_err();
        assert_astroport_pair_contains_denoms(&pair_info, &["uluna", "ukrw"]).unwrap_err();
        assert_astroport_pair_contains_denoms(&pair_info, &["ukrw"]).unwrap_err();
    }
}
