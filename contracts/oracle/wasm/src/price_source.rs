use std::fmt;

use astroport::{
    asset::AssetInfo,
    pair::TWAP_PRECISION,
    querier::{query_token_precision, simulate},
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Deps, Empty, Env, Uint128};
use cw_storage_plus::Map;
use mars_oracle::AstroportTwapSnapshot;
use mars_oracle_base::{
    pyth::PriceIdentifier,
    ContractError::{self},
    ContractResult, PriceSourceChecked, PriceSourceUnchecked,
};

use crate::{
    helpers::{
        add_route_prices, astro_native_asset, period_diff, query_astroport_cumulative_price,
        validate_route_assets,
    },
    state::{ASTROPORT_FACTORY, ASTROPORT_TWAP_SNAPSHOTS},
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
    Pyth {
        /// Contract address of Pyth
        contract_addr: A,

        /// Price feed id of an asset from the list: https://pyth.network/developers/price-feed-ids
        price_feed_id: PriceIdentifier,

        /// The maximum number of seconds since the last price was by an oracle, before
        /// rejecting the price as too stale
        max_staleness: u64,
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
                    "astroport_twap:{pair_address}. Window Size: {window_size}. Tolerance: {tolerance}. Route: {route_str}"
                )
            }
            WasmPriceSource::Pyth {
                contract_addr,
                price_feed_id,
                max_staleness,
            } => format!("pyth:{contract_addr}:{price_feed_id}:{max_staleness}"),
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
            WasmPriceSource::Pyth {
                contract_addr,
                price_feed_id,
                max_staleness,
            } => Ok(WasmPriceSourceChecked::Pyth {
                contract_addr: deps.api.addr_validate(&contract_addr)?,
                price_feed_id,
                max_staleness,
            }),
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
                pair_address,
                window_size,
                tolerance,
                route_assets,
            } => query_astroport_twap_price(
                deps,
                env,
                denom,
                base_denom,
                price_sources,
                pair_address,
                route_assets,
                *window_size,
                *tolerance,
            ),
            WasmPriceSource::Pyth {
                contract_addr,
                price_feed_id,
                max_staleness,
            } => mars_oracle_base::pyth::query_pyth_price(
                deps,
                env,
                contract_addr.clone(),
                *price_feed_id,
                *max_staleness,
            ),
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
    route_assets: &[String],
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

    let price = Decimal::from_ratio(sim_res.return_amount, one);

    // If there are route assets, we need to multiply the price by the price of the
    // route assets in the base denom
    add_route_prices(deps, env, base_denom, price_sources, route_assets, &price)
}

/// Queries the TWAP price of `denom` denominated in `base_denom` from the Astroport pair at `pair_address`.
#[allow(clippy::too_many_arguments)]
fn query_astroport_twap_price(
    deps: &Deps,
    env: &Env,
    denom: &str,
    base_denom: &str,
    price_sources: &Map<&str, WasmPriceSourceChecked>,
    pair_address: &Addr,
    route_assets: &[String],
    window_size: u64,
    tolerance: u64,
) -> ContractResult<Decimal> {
    let snapshots = ASTROPORT_TWAP_SNAPSHOTS
        .may_load(deps.storage, denom)?
        .ok_or(ContractError::NoSnapshots {})?;

    // First, query the current TWAP snapshot
    let current_snapshot = AstroportTwapSnapshot {
        timestamp: env.block.time.seconds(),
        price_cumulative: query_astroport_cumulative_price(&deps.querier, pair_address, denom)?,
    };

    // Find the oldest snapshot whose period from current snapshot is within the tolerable window
    // We do this using a linear search, and quit as soon as we find one; otherwise throw error
    let previous_snapshot = snapshots
        .iter()
        .find(|snapshot| period_diff(&current_snapshot, snapshot, window_size) <= tolerance)
        .ok_or(ContractError::NoSnapshotWithinTolerance {})?;

    // Handle the case if Astroport's cumulative price overflows. In this case, cumulative
    // price wraps back to zero, resulting in more recent cum. prices being smaller than
    // earlier ones.
    //
    // Calculations below assumes the cumulative price doesn't overflows more than once during
    // the period, which should always be the case in practice
    let price_delta = if current_snapshot.price_cumulative >= previous_snapshot.price_cumulative {
        current_snapshot.price_cumulative - previous_snapshot.price_cumulative
    } else {
        current_snapshot
            .price_cumulative
            .checked_add(Uint128::MAX - previous_snapshot.price_cumulative)?
    };
    let period = current_snapshot.timestamp - previous_snapshot.timestamp;
    // NOTE: Astroport introduces TWAP precision (https://github.com/astroport-fi/astroport/pull/143).
    // We need to divide the result by price_precision: (price_delta / (time * price_precision)).
    let price_precision = Uint128::from(10_u128.pow(TWAP_PRECISION.into()));
    let price = Decimal::from_ratio(price_delta, price_precision.checked_mul(period.into())?);

    // If there are route assets, we need to multiply the price by the price of the
    // route assets in the base denom
    add_route_prices(deps, env, base_denom, price_sources, route_assets, &price)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_pyth_price_source() {
        let ps = WasmPriceSourceChecked::Pyth {
            contract_addr: Addr::unchecked("osmo12j43nf2f0qumnt2zrrmpvnsqgzndxefujlvr08"),
            price_feed_id: PriceIdentifier::from_hex(
                "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3",
            )
            .unwrap(),
            max_staleness: 60,
        };
        assert_eq!(
            ps.to_string(),
            "pyth:osmo12j43nf2f0qumnt2zrrmpvnsqgzndxefujlvr08:0x61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3:60"
        )
    }
}
