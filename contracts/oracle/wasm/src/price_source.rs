use std::fmt;

use astroport::{factory::PairType, pair::TWAP_PRECISION, querier::simulate};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Deps, Empty, Env, Uint128};
use cw_storage_plus::Map;
use mars_oracle_base::{
    pyth::PriceIdentifier,
    ContractError::{self},
    ContractResult, PriceSourceChecked, PriceSourceUnchecked,
};
use mars_red_bank_types::oracle::{AstroportTwapSnapshot, Config};

use crate::{
    helpers::{
        adjust_precision, astro_native_asset, get_astroport_pair_denoms,
        get_other_astroport_pair_denom, normalize_price, period_diff,
        query_astroport_cumulative_price, query_astroport_pair_info, query_token_precision,
        validate_astroport_pair_price_source,
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
    },
    Pyth {
        /// Contract address of Pyth
        contract_addr: A,

        /// Price feed id of an asset from the list: https://pyth.network/developers/price-feed-ids
        price_feed_id: PriceIdentifier,

        /// The maximum number of seconds since the last price was by an oracle, before
        /// rejecting the price as too stale
        max_staleness: u64,

        /// Assets are represented in their smallest unit and every asset can have different decimals (e.g. OSMO - 6 decimals, WETH - 18 decimals).
        ///
        /// Pyth prices are denominated in USD so basically it means how much 1 USDC, 1 ATOM, 1 OSMO is worth in USD (NOT 1 uusdc, 1 uatom, 1 uosmo).
        /// We have to normalize it. We should get how much 1 utoken is worth in uusd. For example:
        /// - base_denom = uusd
        /// - price source set for usd (e.g. FIXED price source where 1 usd = 1000000 uusd)
        /// - denom_decimals (ATOM) = 6
        ///
        /// 1 OSMO = 10^6 uosmo
        ///
        /// osmo_price_in_usd = 0.59958994
        /// uosmo_price_in_uusd = osmo_price_in_usd * usd_price_in_base_denom / 10^denom_decimals =
        /// uosmo_price_in_uusd = 0.59958994 * 1000000 * 10^(-6) = 0.59958994
        denom_decimals: u8,
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
            } => {
                format!("astroport_spot:{pair_address}.")
            }
            WasmPriceSource::AstroportTwap {
                pair_address,
                window_size,
                tolerance,
            } => {
                format!(
                    "astroport_twap:{pair_address}. Window Size: {window_size}. Tolerance: {tolerance}."
                )
            }
            WasmPriceSource::Pyth {
                contract_addr,
                price_feed_id,
                max_staleness,
                denom_decimals,
            } => format!("pyth:{contract_addr}:{price_feed_id}:{max_staleness}:{denom_decimals}"),
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
        if denom == base_denom {
            return Err(ContractError::InvalidPriceSource {
                reason: "cannot set price source for base denom".to_string(),
            });
        }

        match self {
            WasmPriceSource::Fixed {
                price,
            } => Ok(WasmPriceSourceChecked::Fixed {
                price,
            }),
            WasmPriceSource::AstroportSpot {
                pair_address,
            } => {
                let pair_address = deps.api.addr_validate(&pair_address)?;

                validate_astroport_pair_price_source(
                    deps,
                    &pair_address,
                    denom,
                    base_denom,
                    price_sources,
                )?;

                Ok(WasmPriceSourceChecked::AstroportSpot {
                    pair_address,
                })
            }
            WasmPriceSource::AstroportTwap {
                pair_address,
                window_size,
                tolerance,
            } => {
                if tolerance >= window_size {
                    return Err(ContractError::InvalidPriceSource {
                        reason: "tolerance must be less than window size".to_string(),
                    });
                }

                let pair_address = deps.api.addr_validate(&pair_address)?;
                validate_astroport_pair_price_source(
                    deps,
                    &pair_address,
                    denom,
                    base_denom,
                    price_sources,
                )?;
                if window_size <= 1 {
                    return Err(ContractError::InvalidPriceSource {
                        reason: "window_size must be greater than 1".to_string(),
                    });
                }

                Ok(WasmPriceSourceChecked::AstroportTwap {
                    pair_address,
                    window_size,
                    tolerance,
                })
            }
            WasmPriceSource::Pyth {
                contract_addr,
                price_feed_id,
                max_staleness,
                denom_decimals,
            } => {
                mars_oracle_base::pyth::assert_usd_price_source(deps, price_sources)?;

                Ok(WasmPriceSourceChecked::Pyth {
                    contract_addr: deps.api.addr_validate(&contract_addr)?,
                    price_feed_id,
                    max_staleness,
                    denom_decimals,
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
        config: &Config,
        price_sources: &Map<&str, Self>,
    ) -> ContractResult<Decimal> {
        match self {
            WasmPriceSource::Fixed {
                price,
            } => Ok(*price),
            WasmPriceSource::AstroportSpot {
                pair_address,
            } => query_astroport_spot_price(deps, env, denom, config, price_sources, pair_address),
            WasmPriceSource::AstroportTwap {
                pair_address,
                window_size,
                tolerance,
            } => query_astroport_twap_price(
                deps,
                env,
                denom,
                config,
                price_sources,
                pair_address,
                *window_size,
                *tolerance,
            ),
            WasmPriceSource::Pyth {
                contract_addr,
                price_feed_id,
                max_staleness,
                denom_decimals,
            } => mars_oracle_base::pyth::query_pyth_price(
                deps,
                env,
                contract_addr.clone(),
                *price_feed_id,
                *max_staleness,
                *denom_decimals,
                config,
                price_sources,
            ),
        }
    }
}

/// Queries the spot price of `denom` denominated in `base_denom` from the Astroport pair at `pair_address`.
fn query_astroport_spot_price(
    deps: &Deps,
    env: &Env,
    denom: &str,
    config: &Config,
    price_sources: &Map<&str, WasmPriceSourceChecked>,
    pair_address: &Addr,
) -> ContractResult<Decimal> {
    let astroport_factory = ASTROPORT_FACTORY.load(deps.storage)?;
    let pair_info = query_astroport_pair_info(&deps.querier, pair_address)?;

    // Get the token's precision
    let p = query_token_precision(&deps.querier, &astroport_factory, denom)?;
    let one = Uint128::new(10_u128.pow(p.into()));

    // Simulate a swap with one unit to get the price. We can't just divide the pools reserves,
    // because that only works for XYK pairs.
    let sim_res = simulate(&deps.querier, pair_address, &astro_native_asset(denom, one))?;

    let price = Decimal::from_ratio(sim_res.return_amount, one);

    normalize_price(deps, env, config, price_sources, &pair_info, denom, price)
}

/// Queries the TWAP price of `denom` denominated in `base_denom` from the Astroport pair at `pair_address`.
#[allow(clippy::too_many_arguments)]
fn query_astroport_twap_price(
    deps: &Deps,
    env: &Env,
    denom: &str,
    config: &Config,
    price_sources: &Map<&str, WasmPriceSourceChecked>,
    pair_address: &Addr,
    window_size: u64,
    tolerance: u64,
) -> ContractResult<Decimal> {
    let snapshots = ASTROPORT_TWAP_SNAPSHOTS
        .may_load(deps.storage, denom)?
        .ok_or(ContractError::NoSnapshots {})?;

    if snapshots.len() < 2 {
        return Err(ContractError::NotEnoughSnapshots {});
    }

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

    // NOTE: Astroport introduces TWAP_PRECISION (https://github.com/astroport-fi/astroport/pull/143).
    // We must adjust the cumulative price delta by the precision factor to get the correct price.
    // For XYK we just need to divide by the TWAP_PRECISION.
    // For StableSwap, the cumulative price is stored as a simulated swap of one unit of the
    // offer asset into the ask asset and then adjusted by the TWAP_PRECISION. So we need to
    // adjust the price delta from TWAP_PRECISION to ask decimals and then divide by one offer unit.
    let pair_info = query_astroport_pair_info(&deps.querier, pair_address)?;

    let price = match pair_info.pair_type {
        PairType::Xyk {} => {
            let price_precision = Uint128::from(10_u128.pow(TWAP_PRECISION.into()));

            Decimal::from_ratio(price_delta, price_precision.checked_mul(period.into())?)
        }
        PairType::Stable {} => {
            // Get the number of decimals of offer and ask denoms
            let pair_denoms = get_astroport_pair_denoms(&pair_info)?;
            let other_pair_denom = get_other_astroport_pair_denom(&pair_denoms, denom)?;
            let astroport_factory = ASTROPORT_FACTORY.load(deps.storage)?;
            let offer_decimals = query_token_precision(&deps.querier, &astroport_factory, denom)?;
            let ask_decimals =
                query_token_precision(&deps.querier, &astroport_factory, &other_pair_denom)?;

            // Adjust the precision of the price delta from TWAP_PRECISION to ask_decimals
            let price_delta = adjust_precision(price_delta, TWAP_PRECISION, ask_decimals)?;

            // Calculate the price by dividing the price delta by the amount of offer asset used in
            // the simulated swap and then multiply by the period
            let offer_simulation_amount = Uint128::from(10_u128.pow(offer_decimals.into()));

            Decimal::from_ratio(price_delta, offer_simulation_amount.checked_mul(period.into())?)
        }
        PairType::Custom(_) => return Err(ContractError::InvalidPairType {}),
    };

    normalize_price(deps, env, config, price_sources, &pair_info, denom, price)
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
            denom_decimals: 6,
        };
        assert_eq!(
            ps.to_string(),
            "pyth:osmo12j43nf2f0qumnt2zrrmpvnsqgzndxefujlvr08:0x61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3:60:6"
        )
    }
}
