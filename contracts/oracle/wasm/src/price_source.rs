use std::fmt;

use astroport::{
    asset::AssetInfo,
    querier::{query_token_precision, simulate},
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Deps, Empty, Env, Uint128};
use cw_storage_plus::Map;
use mars_oracle_base::{
    ContractError::{self},
    ContractResult, PriceSourceChecked, PriceSourceUnchecked,
};

use crate::{
    helpers::{astro_native_asset, validate_route_assets},
    state::ASTROPORT_FACTORY,
};

#[cw_serde]
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
        deps: &Deps,
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
                validate_route_assets(
                    deps,
                    denom,
                    base_denom,
                    price_sources,
                    &pair_address,
                    &route_assets,
                )?;

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
                validate_route_assets(
                    deps,
                    denom,
                    base_denom,
                    price_sources,
                    &pair_address,
                    &route_assets,
                )?;

                //TODO: Validate window_size and tolerance?

                Ok(WasmPriceSourceChecked::AstroportTwap {
                    pair_address: deps.api.addr_validate(&pair_address)?,
                    window_size,
                    tolerance,
                    route_assets,
                })
            }
        }
    }
}

impl PriceSourceChecked<Empty> for WasmPriceSourceChecked {
    #[allow(clippy::only_used_in_recursion)]
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
            } => query_astroport_spot_price(
                deps,
                env,
                denom,
                base_denom,
                price_sources,
                pair_address,
                route_assets,
            ),
            WasmPriceSource::AstroportTwap {
                pair_address: _,
                window_size: _,
                tolerance: _,
                route_assets: _,
            } => todo!(),
        }
    }
}

/// Queries the spot price of `denom` denominated in `base_denom` from the Astroport pair at `pair_address`.
fn query_astroport_spot_price(
    deps: &Deps,
    env: &Env,
    denom: &str,
    base_denom: &str,
    price_sources: &Map<&str, WasmPriceSourceChecked>,
    pair_address: &Addr,
    route_assets: &Vec<String>,
) -> ContractResult<Decimal> {
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

    // Simulate a swap with one unit to get the price. We can't just divide the pools reserves,
    // because that only works for XYK pairs.
    let sim_res = simulate(&deps.querier, pair_address, &astro_native_asset(denom, one))?;

    let mut price = Decimal::from_ratio(sim_res.return_amount, one);

    // If there are route assets, we need to multiply the price by the price of the
    // route assets in the base denom
    for denom in route_assets {
        let price_source =
            price_sources.load(deps.storage, denom).map_err(|_| ContractError::InvalidPrice {
                reason: format!("No price source for route asset {}", denom),
            })?;
        let route_price = price_source.query_price(deps, env, denom, base_denom, price_sources)?;
        price *= route_price;
    }

    Ok(price)
}
