use std::fmt;

use astroport::{
    asset::{AssetInfo},
    querier::{query_token_precision, simulate},
};
use cosmwasm_std::{
    Addr, Decimal, Deps, Empty, Env, Uint128,
};
use cw_storage_plus::Map;
use mars_oracle_base::{
    ContractError::{self},
    ContractResult, PriceSourceChecked, PriceSourceUnchecked,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::helpers::{
    assert_astroport_pair_contains_denoms, astro_native_asset, query_astroport_pair_info,
};
use crate::state::ASTROPORT_FACTORY;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WasmPriceSource<A> {
    /// Returns a fixed value;
    Fixed {
        price: Decimal,
    },
    /// Astroport spot price
    AstroportSpot {
        /// Address of the Astroport pair
        pair_address: A,
        /// Other assets to route through when calculating the price. E.g. if the pair is USDC/ETH
        /// and `config.base_denom` is USD, and we want to get the price of ETH in USD, then
        /// `route_assets` could be `["USDC"]`, which would mean we would get the price of ETH in
        /// USDC, and then multiply by the price of USDC in USD.
        route_assets: Vec<String>,
    },
    /// Astroport TWAP price
    ///
    /// When calculating the  average price, we take the most recent TWAP snapshot and find a second
    /// snapshot in the range of window_size +/- tolerance. For example, if window_size is 5 minutes
    /// and tolerance is 1 minute, we look for snapshots that are 4 - 6 minutes back in time from
    /// the most recent snapshot.
    ///
    /// If there are multiple snapshots within the range, we take the one that is closest to the
    /// desired window size.
    AstroportTwap {
        /// Address of the Astroport pair
        pair_address: A,
        /// The size of the sliding TWAP window in seconds.
        window_size: u64,
        /// The tolerance in seconds for the sliding TWAP window.
        tolerance: u64,
        /// Other assets to route through when calculating the price. E.g. if the pair is USDC/ETH
        /// and `config.base_denom` is USD, and we want to get the price of ETH in USD, then
        /// `route_assets` could be `["USDC"]`, which would mean we would get the price of ETH in
        /// USDC, and then multiply by the price of USDC in USD.
        route_assets: Vec<String>,
    },
}

pub type WasmPriceSourceUnchecked = WasmPriceSource<String>;
pub type WasmPriceSourceChecked = WasmPriceSource<Addr>;

impl fmt::Display for WasmPriceSourceChecked {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let label = match self {
            WasmPriceSource::Fixed {
                price,
            } => format!("fixed:{price}"),
            WasmPriceSource::AstroportSpot {
                pair_address,
                route_assets,
            } => {
                let route_str = route_assets.join(",");
                format!("astroport_spot:{pair_address}. Route: {route_str}")
            }
            WasmPriceSource::AstroportTwap {
                pair_address,
                window_size,
                tolerance,
                route_assets,
            } => {
                let route_str = route_assets.join(",");
                format!(
                    "astroport_twap:{pair_address}:{window_size}:{tolerance}. Route: {route_str}"
                )
            }
        };
        write!(f, "{label}")
    }
}

impl PriceSourceUnchecked<WasmPriceSourceChecked, Empty> for WasmPriceSourceUnchecked {
    fn validate(
        self,
        deps: Deps,
        denom: &str,
        base_denom: &str,
        price_sources: &Map<&str, WasmPriceSourceChecked>,
    ) -> ContractResult<WasmPriceSourceChecked> {
        match self {
            WasmPriceSource::Fixed {
                price,
            } => Ok(WasmPriceSourceChecked::Fixed {
                price,
            }),
            WasmPriceSource::AstroportSpot {
                pair_address,
                route_assets,
            } => {
                // For all route assets, there must be a price source available
                for asset in &route_assets {
                    price_sources.load(deps.storage, asset).map_err(|_| {
                        ContractError::InvalidPriceSource {
                            reason: format!("No price source found for asset {}", asset),
                        }
                    })?;
                }

                // If there are no route assets, then the pair must contain the denom and base denom.
                if route_assets.is_empty() {
                    let pair_info = query_astroport_pair_info(&deps.querier, &pair_address)?;
                    assert_astroport_pair_contains_denoms(&pair_info, &[denom, base_denom])?;
                } else {
                    // If there are route assets, the pair must contain the denom and the first
                    // route asset, and the last route asset must be the base denom.
                    let pair_info = query_astroport_pair_info(&deps.querier, &pair_address)?;
                    assert_astroport_pair_contains_denoms(&pair_info, &[denom, &route_assets[0]])?;

                    // TODO: Is this necessary? As 1 base_denom = 1 base_denom
                    if route_assets.last().unwrap() != base_denom {
                        return Err(ContractError::InvalidPriceSource {
                            reason: format!(
                                "Last route asset {} must be the base denom {}",
                                route_assets.last().unwrap(),
                                base_denom
                            ),
                        });
                    }
                }

                Ok(WasmPriceSourceChecked::AstroportSpot {
                    pair_address: deps.api.addr_validate(&pair_address)?,
                    route_assets,
                })
            }
            WasmPriceSource::AstroportTwap {
                pair_address,
                window_size,
                tolerance,
                route_assets,
            } => {
                let pair_info = query_astroport_pair_info(&deps.querier, pair_address)?;
                assert_astroport_pair_contains_denoms(&pair_info, &[denom, base_denom])?;
                Ok(WasmPriceSourceChecked::AstroportTwap {
                    pair_address: pair_info.contract_addr,
                    window_size,
                    tolerance,
                    route_assets,
                })

                //TODO: Validate window_size and tolerance?
            }
        }
    }
}

impl PriceSourceChecked<Empty> for WasmPriceSourceChecked {
    fn query_price(
        &self,
        deps: &Deps,
        env: &Env,
        denom: &str,
        base_denom: &str,
        price_sources: &Map<&str, Self>,
    ) -> ContractResult<Decimal> {
        match self {
            WasmPriceSource::Fixed {
                price,
            } => Ok(*price),
            WasmPriceSource::AstroportSpot {
                pair_address,
                route_assets,
            } => {
                let astroport_factory = ASTROPORT_FACTORY.load(deps.storage)?;

                // Get the token's precision
                let p = query_token_precision(
                    &deps.querier,
                    &AssetInfo::NativeToken {
                        denom: denom.to_string(),
                    },
                    &astroport_factory,
                )?;
                let one = Uint128::new(10_u128.pow(p.into()));

                // Simulate a swap with one unit to get the price. We can't just divide the pools reserve,
                // because that only works for XYK pairs.
                let sim_res =
                    simulate(&deps.querier, pair_address, &astro_native_asset(denom, one))?;

                let mut price = Decimal::from_ratio(sim_res.return_amount, one);

                // If there are route assets, we need to multiply the price by the price of the
                // route assets in the base denom
                for denom in route_assets {
                    let price_source = price_sources.load(deps.storage, denom).map_err(|_| ContractError::InvalidPrice {
                            reason: format!("No price source for route asset {}", denom),
                        })?;
                    let route_price =
                        price_source.query_price(deps, env, denom, base_denom, price_sources)?;
                    price *= route_price;
                }

                Ok(price)
            }
            WasmPriceSource::AstroportTwap {
                pair_address: _,
                window_size: _,
                tolerance: _,
                route_assets: _,
            } => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use mars_testing::{mock_dependencies};

    use super::*;

    #[test]
    fn display_fixed_price_source() {
        let ps = WasmPriceSource::Fixed {
            price: Decimal::from_ratio(1u128, 2u128),
        };
        assert_eq!(ps.to_string(), "fixed:0.5")
    }

    #[test]
    fn display_spot_price_source() {
        let ps = WasmPriceSourceChecked::AstroportSpot {
            pair_address: Addr::unchecked("fake_addr"),
            route_assets: vec![],
        };
        assert_eq!(ps.to_string(), "astroport_spot:fake_addr. Route: ")
    }

    #[test]
    fn display_spot_price_source_with_route() {
        let ps = WasmPriceSourceChecked::AstroportSpot {
            pair_address: Addr::unchecked("fake_addr"),
            route_assets: vec!["fake_asset1".to_string(), "fake_asset2".to_string()],
        };
        assert_eq!(ps.to_string(), "astroport_spot:fake_addr. Route: fake_asset1,fake_asset2")
    }

    // TODO: Display test for twap

    #[test]
    fn validate_fixed_price_source() {
        let ps = WasmPriceSource::Fixed {
            price: Decimal::from_ratio(1u128, 2u128),
        };
        let deps = mock_dependencies(&[]);
        let price_sources = Map::new("price_sources");
        let denom = "uusd";
        let base_denom = "uusd";
        let res = ps.validate(deps.as_ref(), denom, base_denom, &price_sources);
        assert!(res.is_ok());
    }
}
