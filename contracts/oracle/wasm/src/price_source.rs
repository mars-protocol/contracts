use std::{
    cmp::{min, Ordering},
    fmt,
};

use astroport_v5::{
    asset::PairInfo, factory::PairType, pair::TWAP_PRECISION,
    pair_concentrated::ConcentratedPoolParams, querier::simulate,
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{from_json, Addr, Decimal, Deps, Empty, Env, Uint128};
use cw_storage_plus::Map;
use mars_oracle_base::{
    lp_pricing,
    redemption_rate::{assert_rr_not_too_old, query_redemption_rate, RedemptionRate},
    ContractError, ContractResult, PriceSourceChecked, PriceSourceUnchecked,
};
use mars_types::oracle::{ActionKind, AstroportTwapSnapshot, Config};
use pyth_sdk_cw::PriceIdentifier;

use crate::{
    helpers::{
        adjust_precision, astro_native_asset, get_astroport_pair_denoms,
        get_other_astroport_pair_denom, normalize_price, period_diff, query_astroport_config,
        query_astroport_cumulative_price, query_astroport_pair_info,
        query_astroport_pcl_curve_invariant, query_astroport_pool,
        query_astroport_ss_curve_invariant, query_token_precision,
        validate_astroport_lp_pool_for_type, validate_astroport_pair_price_source,
    },
    lp_pricing::{query_pcl_lp_price, query_stable_swap_lp_price},
    state::{ASTROPORT_FACTORY, ASTROPORT_TWAP_SNAPSHOTS},
};

pub const PRICE_PRECISION: Uint128 = Uint128::new(10_u128.pow(TWAP_PRECISION as u32));

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
        /// We can't verify what denoms consist of the price feed.
        /// Be very careful when adding it !!!
        price_feed_id: PriceIdentifier,

        /// The maximum number of seconds since the last price was by an oracle, before
        /// rejecting the price as too stale
        max_staleness: u64,

        /// The maximum confidence deviation allowed for an oracle price.
        ///
        /// The confidence is measured as the percent of the confidence interval
        /// value provided by the oracle as compared to the weighted average value
        /// of the price.
        max_confidence: Decimal,

        /// The maximum deviation (percentage) between current and EMA price
        max_deviation: Decimal,

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
    /// Liquid Staking Derivatives (LSD) price quoted in USD based on data from Pyth, Astroport and Redemption Rate provider (e.g. Stride).
    ///
    /// Equation to calculate the price:
    /// stAsset/USD = stAsset/Asset * Asset/USD
    /// where:
    /// stAsset/Asset = min(stAsset/Asset TWAP, stAsset/Asset Redemption Rate)
    ///
    /// Example:
    /// stATOM/USD = stATOM/ATOM * ATOM/USD
    /// where:
    /// - stATOM/ATOM = min(stAtom/Atom TWAP from Astroport, stAtom/Atom Redemption Rate)
    /// - ATOM/USD price comes from the Mars Oracle contract (should point to Pyth).
    Lsd {
        /// Transitive denom for which we query price in USD. It refers to 'Asset' in the equation:
        /// stAsset/USD = stAsset/Asset * Asset/USD
        transitive_denom: String,

        /// Params to query TWAP price
        twap: AstroportTwap<A>,

        /// Params to query redemption rate
        redemption_rate: RedemptionRate<A>,
    },
    /// Astroport LP token (of an XYK pool) price quoted in uusd
    XykLiquidityToken {
        /// Address of the Astroport pair
        pair_address: A,
    },
    PclLiquidityToken {
        /// Address of the Astroport pair
        pair_address: A,
    },
    SsLiquidityToken {
        /// Address of the Astroport pair
        pair_address: A,
    },
}

#[cw_serde]
pub struct AstroportTwap<A> {
    /// Address of the Astroport pair
    pub pair_address: A,
    /// The size of the sliding TWAP window in seconds.
    pub window_size: u64,
    /// The tolerance in seconds for the sliding TWAP window.
    pub tolerance: u64,
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
                max_confidence,
                max_deviation,
                denom_decimals
            } => format!("pyth:{contract_addr}:{price_feed_id}:{max_staleness}:{max_confidence}:{max_deviation}:{denom_decimals}"),
            WasmPriceSource::Lsd { transitive_denom, twap, redemption_rate } => {
                let AstroportTwap {
                    pair_address,
                    window_size,
                    tolerance,
                } = twap;
                let RedemptionRate {
                    contract_addr,
                    max_staleness,
                } = redemption_rate;
                format!("lsd:{transitive_denom}:{pair_address}:{window_size}:{tolerance}:{contract_addr}:{max_staleness}")
            },
            WasmPriceSource::XykLiquidityToken { pair_address } => format!("xyk_liquidity_token:{pair_address}"),
            WasmPriceSource::PclLiquidityToken { pair_address } => format!("pcl_liquidity_token:{pair_address}"),
            WasmPriceSource::SsLiquidityToken { pair_address } => format!("stable_swap_liquidity_token:{pair_address}"),
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
                max_confidence,
                max_deviation,
                denom_decimals,
            } => {
                mars_oracle_base::pyth::assert_pyth(max_confidence, max_deviation, denom_decimals)?;
                mars_oracle_base::pyth::assert_usd_price_source(deps, price_sources)?;
                Ok(WasmPriceSourceChecked::Pyth {
                    contract_addr: deps.api.addr_validate(&contract_addr)?,
                    price_feed_id,
                    max_staleness,
                    max_confidence,
                    max_deviation,
                    denom_decimals,
                })
            }
            WasmPriceSource::Lsd {
                transitive_denom,
                twap,
                redemption_rate,
            } => {
                if !price_sources.has(deps.storage, &transitive_denom) {
                    return Err(ContractError::InvalidPriceSource {
                        reason: format!("missing price source for {}", transitive_denom),
                    });
                }

                let AstroportTwap {
                    pair_address,
                    window_size,
                    tolerance,
                } = twap.clone();
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
                    &transitive_denom,
                    price_sources,
                )?;
                if window_size <= 1 {
                    return Err(ContractError::InvalidPriceSource {
                        reason: "window_size must be greater than 1".to_string(),
                    });
                }

                Ok(WasmPriceSourceChecked::Lsd {
                    transitive_denom,
                    twap: AstroportTwap {
                        pair_address,
                        window_size,
                        tolerance,
                    },
                    redemption_rate: RedemptionRate {
                        contract_addr: deps.api.addr_validate(&redemption_rate.contract_addr)?,
                        max_staleness: redemption_rate.max_staleness,
                    },
                })
            }
            WasmPriceSource::XykLiquidityToken {
                pair_address,
            } => {
                let pair_address = deps.api.addr_validate(&pair_address)?;
                validate_astroport_lp_pool_for_type(
                    deps,
                    &pair_address,
                    price_sources,
                    PairType::Xyk {},
                )?;

                Ok(WasmPriceSourceChecked::XykLiquidityToken {
                    pair_address,
                })
            }
            WasmPriceSource::PclLiquidityToken {
                pair_address,
            } => {
                let pair_address = deps.api.addr_validate(&pair_address)?;
                validate_astroport_lp_pool_for_type(
                    deps,
                    &pair_address,
                    price_sources,
                    PairType::Custom("concentrated".to_string()),
                )?;

                Ok(WasmPriceSourceChecked::PclLiquidityToken {
                    pair_address,
                })
            }
            WasmPriceSource::SsLiquidityToken {
                pair_address,
            } => {
                let pair_address = deps.api.addr_validate(&pair_address)?;
                validate_astroport_lp_pool_for_type(
                    deps,
                    &pair_address,
                    price_sources,
                    PairType::Stable {},
                )?;

                Ok(WasmPriceSourceChecked::SsLiquidityToken {
                    pair_address,
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
        kind: ActionKind,
    ) -> ContractResult<Decimal> {
        match self {
            WasmPriceSource::Fixed {
                price,
            } => Ok(*price),
            WasmPriceSource::AstroportSpot {
                pair_address,
            } => query_astroport_spot_price(
                deps,
                env,
                denom,
                config,
                price_sources,
                pair_address,
                kind,
            ),
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
                kind,
                true,
            ),
            WasmPriceSource::Pyth {
                contract_addr,
                price_feed_id,
                max_staleness,
                max_confidence,
                max_deviation,
                denom_decimals,
            } => mars_oracle_base::pyth::query_pyth_price(
                deps,
                env,
                contract_addr.clone(),
                *price_feed_id,
                *max_staleness,
                *max_confidence,
                *max_deviation,
                *denom_decimals,
                config,
                price_sources,
                kind,
            ),
            WasmPriceSource::Lsd {
                transitive_denom,
                twap,
                redemption_rate,
            } => query_lsd_price(
                deps,
                env,
                denom,
                transitive_denom,
                twap,
                redemption_rate,
                config,
                price_sources,
                kind,
            ),
            WasmPriceSource::XykLiquidityToken {
                pair_address,
            } => query_xyk_liquidity_token_price(
                deps,
                env,
                config,
                price_sources,
                pair_address,
                kind,
            ),
            WasmPriceSource::PclLiquidityToken {
                pair_address,
            } => query_pcl_liquidity_token_price(
                deps,
                env,
                config,
                price_sources,
                pair_address,
                kind,
            ),
            WasmPriceSource::SsLiquidityToken {
                pair_address,
            } => {
                query_ss_liquidity_token_price(deps, env, config, price_sources, pair_address, kind)
            }
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
    kind: ActionKind,
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

    normalize_price(deps, env, config, price_sources, &pair_info, denom, price, kind)
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
    kind: ActionKind,
    normalize: bool,
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
    let price_delta = match current_snapshot
        .price_cumulative
        .cmp(&previous_snapshot.price_cumulative)
    {
        Ordering::Greater => current_snapshot.price_cumulative - previous_snapshot.price_cumulative,
        Ordering::Less => current_snapshot
            .price_cumulative
            .checked_add(Uint128::MAX - previous_snapshot.price_cumulative)?,
        Ordering::Equal => {
            // This should never happen since cumulative price is monotonically increasing, but we throw
            // here just in case, rather than returning a zero price.
            return Err(ContractError::InvalidCumulativePrice {});
        }
    };
    let period = current_snapshot.timestamp - previous_snapshot.timestamp;

    let pair_info = query_astroport_pair_info(&deps.querier, pair_address)?;
    // NOTE: Astroport introduces TWAP_PRECISION (https://github.com/astroport-fi/astroport/pull/143).
    // We must adjust the cumulative price delta by the precision factor to get the correct price.
    let price = match pair_info.pair_type {
        // For XYK we just need to divide by the TWAP_PRECISION as the number of decimals for each asset
        // is disregarded in the calculations.
        PairType::Xyk {} => {
            Decimal::from_ratio(price_delta, PRICE_PRECISION.checked_mul(period.into())?)
        }
        // For StableSwap, the cumulative price is stored as a simulated swap of one unit of the
        // offer asset into the ask asset and then adjusted by the TWAP_PRECISION. So we need to
        // adjust the price delta from TWAP_PRECISION to ask decimals and then divide by one offer unit.
        // E.g. for a stableswap pool with 5 decimals for the offer asset and 7 decimals for ask:
        // Lets assume the price source denom is ATOM and the ask denom is OSMO. Further assume:
        // ATOM: 5 decimals
        // OSMO: 8 decimals
        // TWAP_PRECISION: 6 decimals
        // 1 ATOM = 10^5 uatom
        // 1 OSMO = 10^8 uosmo
        // Pool contains: 1000 ATOM and 1000 OSMO, i.e. 10^8 uatom and 10^11 uosmo, this means that
        // the price is 1:1 ATOM:OSMO, or 1:1000 uatom:uosmo.
        //
        // When calculating the cumulative price, astroport simulates a swap of 1 base unit of the
        // offer asset into the ask asset. In our example, this means 10^5 uatom is swapped into
        // 10^8 uosmo. This is then adjusted by the TWAP_PRECISION:
        // cumulative_price = swap_return_amount / 10^(OSMO - DECIMALS - TWAP_PRECISION) = 10^8 / 10^(8-6) = 10^6.
        // In order to convert this back into a price of uosmo/uatom, we need to reverse the
        // process. So we `adjust_precision` back from TWAP_PRECISION to 8 decimals, which performs
        // cumulative_price * 10^(OSMO_DECIMALS - TWAP_DECIMALS) = 10^6 * 10^(8-6) = 10^8.
        // This is then divided by one base unit of ATOM (10^5) to get the
        // final price of 10^3 uosmo/uatom.
        PairType::Stable {} => {
            // Get the number of decimals of offer and ask denoms
            let (offer_decimals, ask_decimals) = get_precisions(deps, &pair_info, denom)?;

            // Adjust the precision of the price delta from TWAP_PRECISION to ask_decimals
            let price_delta = adjust_precision(price_delta, TWAP_PRECISION, ask_decimals)?;

            // Calculate the price by dividing the price delta by the amount of offer asset used in
            // the simulated swap and then multiply by the period
            let offer_simulation_amount = Uint128::from(10_u128.pow(offer_decimals.into()));

            Decimal::from_ratio(price_delta, offer_simulation_amount.checked_mul(period.into())?)
        }
        PairType::Custom(ref custom) if custom == "concentrated" => {
            // Get the number of decimals of offer and ask denoms
            let (offer_decimals, ask_decimals) = get_precisions(deps, &pair_info, denom)?;

            let denominator = PRICE_PRECISION.checked_mul(period.into())?;

            // price = (price_delta / (10^TWAP_PRECISION * period)) * (10^ask_decimals / 10^offer_decimals)
            apply_decimals_to_price(price_delta, denominator, ask_decimals, offer_decimals)?
        }
        PairType::Custom(_) => return Err(ContractError::InvalidPairType {}),
    };

    if normalize {
        normalize_price(deps, env, config, price_sources, &pair_info, denom, price, kind)
    } else {
        Ok(price)
    }
}

fn get_precisions(
    deps: &Deps,
    pair_info: &PairInfo,
    denom: &str,
) -> Result<(u8, u8), ContractError> {
    let pair_denoms = get_astroport_pair_denoms(pair_info)?;
    let other_pair_denom = get_other_astroport_pair_denom(&pair_denoms, denom)?;
    let astroport_factory = ASTROPORT_FACTORY.load(deps.storage)?;
    let offer_decimals = query_token_precision(&deps.querier, &astroport_factory, denom)?;
    let ask_decimals = query_token_precision(&deps.querier, &astroport_factory, &other_pair_denom)?;
    Ok((offer_decimals, ask_decimals))
}

/// Applies the decimals of the offer and ask assets to the price.
/// The price is calculated as `(price_delta / (10^TWAP_PRECISION * period)) * (10^ask_decimals / 10^offer_decimals)`.
fn apply_decimals_to_price(
    nominator: Uint128,
    denominator: Uint128,
    ask_decimals: u8,
    offer_decimals: u8,
) -> Result<Decimal, ContractError> {
    Ok(match ask_decimals.cmp(&offer_decimals) {
        // If the decimals are equal, we can just divide the nominator by the denominator
        Ordering::Equal => Decimal::from_ratio(nominator, denominator),
        // If the ask decimals are lower than the offer decimals, we need to multiply the denominator by
        // 10^(offer_decimals - ask_decimals)
        // E.g. if the decimals are 6 and 8, we need to multiply the denominator by 10^(8 - 6) = 10^2
        Ordering::Less => {
            let denominator = denominator
                .checked_mul(Uint128::new(10_u128.pow((offer_decimals - ask_decimals) as u32)))?;
            Decimal::from_ratio(nominator, denominator)
        }
        // If the ask decimals are higher than the offer decimals, we need to multiply the nominator by
        // 10^(ask_decimals - offer_decimals)
        // E.g. if the decimals are 8 and 6, we need to multiply the nominator by 10^(8 - 6) = 10^2
        Ordering::Greater => {
            let nominator = nominator
                .checked_mul(Uint128::new(10_u128.pow((ask_decimals - offer_decimals) as u32)))?;
            Decimal::from_ratio(nominator, denominator)
        }
    })
}

/// Staked asset price quoted in USD.
///
/// stAsset/USD = stAsset/Asset * Asset/USD
/// where:
/// stAsset/Asset = min(stAsset/Asset TWAP, stAsset/Asset Redemption Rate)
#[allow(clippy::too_many_arguments)]
fn query_lsd_price(
    deps: &Deps,
    env: &Env,
    denom: &str,
    transitive_denom: &str,
    twap: &AstroportTwap<Addr>,
    redemption_rate: &RedemptionRate<Addr>,
    config: &Config,
    price_sources: &Map<&str, WasmPriceSourceChecked>,
    kind: ActionKind,
) -> ContractResult<Decimal> {
    let current_time = env.block.time.seconds();

    let staked_price = query_astroport_twap_price(
        deps,
        env,
        denom,
        config,
        price_sources,
        &twap.pair_address,
        twap.window_size,
        twap.tolerance,
        kind.clone(),
        false, // we don't want to normalize the price to be in base_denom
    )?;

    // query redemption rate
    let rr = query_redemption_rate(
        &deps.querier,
        redemption_rate.contract_addr.clone(),
        denom.to_string(),
    )?;

    // Check if the redemption rate is not too old
    assert_rr_not_too_old(current_time, &rr, redemption_rate)?;

    // min from geometric TWAP and exchange rate
    let min_price = min(staked_price, rr.redemption_rate);

    // use current price source
    let transitive_price = price_sources.load(deps.storage, transitive_denom)?.query_price(
        deps,
        env,
        transitive_denom,
        config,
        price_sources,
        kind,
    )?;

    min_price.checked_mul(transitive_price).map_err(Into::into)
}

fn query_xyk_liquidity_token_price(
    deps: &Deps,
    env: &Env,
    config: &Config,
    price_sources: &Map<&str, WasmPriceSourceChecked>,
    pair_address: &Addr,
    kind: ActionKind,
) -> ContractResult<Decimal> {
    // XYK pool asserted during price source creation
    let pool = query_astroport_pool(&deps.querier, pair_address)?;

    let coin0 = pool.assets[0].as_coin()?;
    let coin1 = pool.assets[1].as_coin()?;

    lp_pricing::query_xyk_lp_price(
        deps,
        env,
        config,
        price_sources,
        kind,
        coin0,
        coin1,
        pool.total_share,
    )
}

fn query_pcl_liquidity_token_price(
    deps: &Deps,
    env: &Env,
    config: &Config,
    price_sources: &Map<&str, WasmPriceSourceChecked>,
    pair_address: &Addr,
    kind: ActionKind,
) -> ContractResult<Decimal> {
    // PCL pool asserted during price source creation
    let pool = query_astroport_pool(&deps.querier, pair_address)?;
    let coin0 = pool.assets[0].as_coin()?;
    let coin1 = pool.assets[1].as_coin()?;

    let pool_config = query_astroport_config(&deps.querier, pair_address)?;
    let pool_params = match pool_config.params {
        Some(params) => from_json::<ConcentratedPoolParams>(params)?,
        None => return Err(ContractError::MissingAstroportPoolParams {}),
    };

    let curve_invariant = query_astroport_pcl_curve_invariant(&deps.querier, pair_address)?;

    query_pcl_lp_price(
        deps,
        env,
        config,
        price_sources,
        kind,
        coin0,
        coin1,
        pool.total_share,
        pool_params.price_scale,
        curve_invariant,
    )
}

fn query_ss_liquidity_token_price(
    deps: &Deps,
    env: &Env,
    config: &Config,
    price_sources: &Map<&str, WasmPriceSourceChecked>,
    pair_address: &Addr,
    kind: ActionKind,
) -> ContractResult<Decimal> {
    // StableSwap pool asserted during price source creation
    let pool = query_astroport_pool(&deps.querier, pair_address)?;
    let coin0 = pool.assets[0].as_coin()?;
    let coin1 = pool.assets[1].as_coin()?;

    let curve_invariant = query_astroport_ss_curve_invariant(&deps.querier, pair_address)?;

    query_stable_swap_lp_price(
        deps,
        env,
        config,
        price_sources,
        kind,
        coin0,
        coin1,
        pool.total_share,
        curve_invariant,
    )
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

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
            max_confidence: Decimal::percent(10u64),
            max_deviation: Decimal::percent(15u64),
            denom_decimals: 18,
        };
        assert_eq!(
                ps.to_string(),
                "pyth:osmo12j43nf2f0qumnt2zrrmpvnsqgzndxefujlvr08:0x61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3:60:0.1:0.15:18"
            )
    }

    #[test]
    fn concentrated_price_if_equal_decimals() {
        let nominator = Uint128::new(10_u128.pow(6));
        let denominator = Uint128::new(10_u128.pow(6));
        let ask_decimals = 6;
        let offer_decimals = 6;
        let price =
            apply_decimals_to_price(nominator, denominator, ask_decimals, offer_decimals).unwrap();
        assert_eq!(price, Decimal::one());
    }

    #[test]
    fn concentrated_price_if_ask_decimals_less_than_offer_decimals() {
        let nominator = Uint128::new(10_u128.pow(6));
        let denominator = Uint128::new(10_u128.pow(6));
        let ask_decimals = 6;
        let offer_decimals = 8;
        let price =
            apply_decimals_to_price(nominator, denominator, ask_decimals, offer_decimals).unwrap();
        assert_eq!(price, Decimal::from_ratio(1u128, 100u128));

        // simulate calculation with big different between decimals, for example DyDx/USDC pair
        let two_days_sec = Uint128::new(172800);
        let nominator = Uint128::new(12_000_000_000_000);
        let denominator = PRICE_PRECISION * two_days_sec;
        let ask_decimals = 6;
        let offer_decimals = 18;
        let price =
            apply_decimals_to_price(nominator, denominator, ask_decimals, offer_decimals).unwrap();
        assert_eq!(price, Decimal::from_str("0.000000000069444444").unwrap());
    }

    #[test]
    fn concentrated_price_if_ask_decimals_greater_than_offer_decimals() {
        let nominator = Uint128::new(10_u128.pow(6));
        let denominator = Uint128::new(10_u128.pow(6));
        let ask_decimals = 8;
        let offer_decimals = 6;
        let price =
            apply_decimals_to_price(nominator, denominator, ask_decimals, offer_decimals).unwrap();
        assert_eq!(price, Decimal::from_ratio(100u128, 1u128));

        // simulate calculation with big different between decimals, for example DyDx/USDC pair
        let two_days_sec = Uint128::new(172800);
        let nominator = Uint128::new(12_000_000_000_000);
        let denominator = PRICE_PRECISION * two_days_sec;
        let ask_decimals = 18;
        let offer_decimals = 6;
        let price =
            apply_decimals_to_price(nominator, denominator, ask_decimals, offer_decimals).unwrap();
        assert_eq!(price, Decimal::from_str("69444444444444.444444444444444444").unwrap());
    }
}
