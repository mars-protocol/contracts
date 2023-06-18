use std::{cmp::min, fmt};

use cosmwasm_std::{Addr, Decimal, Decimal256, Deps, Empty, Env, Isqrt, Uint128, Uint256};
use cw_storage_plus::Map;
use mars_oracle_base::{
    ContractError::InvalidPrice, ContractResult, PriceSourceChecked, PriceSourceUnchecked,
};
use mars_osmosis::helpers::{
    query_arithmetic_twap_price, query_geometric_twap_price, query_pool, query_spot_price,
    recovered_since_downtime_of_length, Pool,
};
use mars_red_bank_types::oracle::msg::{ActionKind, Config};
use pyth_sdk_cw::PriceIdentifier;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{helpers, stride::query_redemption_rate};

/// Copied from https://github.com/osmosis-labs/osmosis-rust/blob/main/packages/osmosis-std/src/types/osmosis/downtimedetector/v1beta1.rs#L4
///
/// It doesn't impl Serialize, Deserialize, and JsonSchema traits, and therefore
/// cannot be used in contract APIs (messages and query responses).
///
/// TODO: Make a PR to osmosis-rust that implements these traits for enum types.
/// Once merged, remove this one here.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Downtime {
    Duration30s = 0,
    Duration1m = 1,
    Duration2m = 2,
    Duration3m = 3,
    Duration4m = 4,
    Duration5m = 5,
    Duration10m = 6,
    Duration20m = 7,
    Duration30m = 8,
    Duration40m = 9,
    Duration50m = 10,
    Duration1h = 11,
    Duration15h = 12,
    Duration2h = 13,
    Duration25h = 14,
    Duration3h = 15,
    Duration4h = 16,
    Duration5h = 17,
    Duration6h = 18,
    Duration9h = 19,
    Duration12h = 20,
    Duration18h = 21,
    Duration24h = 22,
    Duration36h = 23,
    Duration48h = 24,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct DowntimeDetector {
    /// Downtime period options that you can query, to be: 30seconds, 1 min, 2 min, 3 min, 4 min,
    /// 5 min, 10 min, 20 min, 30 min, 40 min, 50 min, 1 hr, 1.5hr, 2 hr, 2.5 hr, 3 hr, 4 hr, 5 hr,
    /// 6 hr, 9hr, 12hr, 18hr, 24hr, 36hr, 48hr.
    pub downtime: Downtime,

    /// Recovery seconds since the chain has been down for downtime period.
    pub recovery: u64,
}

impl DowntimeDetector {
    fn fmt(opt_dd: &Option<Self>) -> String {
        match opt_dd {
            None => "None".to_string(),
            Some(dd) => format!("Some({dd})"),
        }
    }
}

impl fmt::Display for DowntimeDetector {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}:{}", self.downtime, self.recovery)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OsmosisPriceSource<T> {
    /// Returns a fixed value;
    Fixed {
        price: Decimal,
    },
    /// Osmosis spot price quoted in OSMO
    ///
    /// NOTE: `pool_id` must point to an Osmosis pool consists of the asset of interest and OSMO
    Spot {
        pool_id: u64,
    },
    /// Osmosis arithmetic twap price quoted in OSMO
    ///
    /// NOTE: `pool_id` must point to an Osmosis pool consists of the asset of interest and OSMO
    ArithmeticTwap {
        pool_id: u64,

        /// Window size in seconds representing the entire window for which 'average' price is calculated.
        /// Value should be <= 172800 sec (48 hours).
        window_size: u64,

        /// Detect when the chain is recovering from downtime
        downtime_detector: Option<DowntimeDetector>,
    },
    /// Osmosis geometric twap price quoted in OSMO
    ///
    /// NOTE: `pool_id` must point to an Osmosis pool consists of the asset of interest and OSMO
    GeometricTwap {
        pool_id: u64,

        /// Window size in seconds representing the entire window for which 'geometric' price is calculated.
        /// Value should be <= 172800 sec (48 hours).
        window_size: u64,

        /// Detect when the chain is recovering from downtime
        downtime_detector: Option<DowntimeDetector>,
    },
    /// Osmosis LP token (of an XYK pool) price quoted in OSMO
    XykLiquidityToken {
        pool_id: u64,
    },
    /// Osmosis geometric twap price quoted in OSMO for staked asset.
    ///
    /// Equation to calculate the price:
    /// stAsset/OSMO = stAsset/Asset * Asset/OSMO
    ///
    /// Example:
    /// stATOM/OSMO = stATOM/ATOM * ATOM/OSMO
    /// where:
    /// - stATOM/ATOM price calculated using the geometric TWAP from the stATOM/ATOM pool.
    /// - ATOM/OSMO price comes from the Mars Oracle contract.
    ///
    /// NOTE: `pool_id` must point to stAsset/Asset Osmosis pool.
    /// Asset/OSMO price source should be available in the Mars Oracle contract.
    StakedGeometricTwap {
        /// Transitive denom for which we query price in OSMO. It refers to 'Asset' in the equation:
        /// stAsset/OSMO = stAsset/Asset * Asset/OSMO
        transitive_denom: String,

        /// Pool id for stAsset/Asset pool
        pool_id: u64,

        /// Window size in seconds representing the entire window for which 'geometric' price is calculated.
        /// Value should be <= 172800 sec (48 hours).
        window_size: u64,

        /// Detect when the chain is recovering from downtime
        downtime_detector: Option<DowntimeDetector>,
    },
    Pyth {
        /// Contract address of Pyth
        contract_addr: T,

        /// Price feed id of an asset from the list: https://pyth.network/developers/price-feed-ids
        /// We can't verify what denoms consist of the price feed.
        /// Be very careful when adding it !!!
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
    /// Liquid Staking Derivatives (LSD) price quoted in USD based on data from Pyth, Osmosis and Stride.
    ///
    /// Equation to calculate the price:
    /// stAsset/USD = stAsset/Asset * Asset/USD
    /// where:
    /// stAsset/Asset = min(stAsset/Asset Geometric TWAP, stAsset/Asset Redemption Rate)
    ///
    /// Example:
    /// stATOM/USD = stATOM/ATOM * ATOM/USD
    /// where:
    /// - stATOM/ATOM = min(stAtom/Atom Geometric TWAP from Osmosis, stAtom/Atom Redemption Rate from Stride)
    /// - ATOM/USD price comes from the Mars Oracle contract (should point to Pyth).
    ///
    /// NOTE: `pool_id` must point to stAsset/Asset Osmosis pool.
    /// Asset/USD price source should be available in the Mars Oracle contract.
    Lsd {
        /// Transitive denom for which we query price in USD. It refers to 'Asset' in the equation:
        /// stAsset/USD = stAsset/Asset * Asset/USD
        transitive_denom: String,

        /// Params to query geometric TWAP price
        geometric_twap: GeometricTwap,

        /// Params to query redemption rate
        redemption_rate: RedemptionRate<T>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GeometricTwap {
    /// Pool id for stAsset/Asset pool
    pub pool_id: u64,

    /// Window size in seconds representing the entire window for which 'geometric' price is calculated.
    /// Value should be <= 172800 sec (48 hours).
    pub window_size: u64,

    /// Detect when the chain is recovering from downtime
    pub downtime_detector: Option<DowntimeDetector>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RedemptionRate<T> {
    /// Contract addr
    pub contract_addr: T,

    /// The maximum number of seconds since the last price was by an oracle, before
    /// rejecting the price as too stale
    pub max_staleness: u64,
}

pub type OsmosisPriceSourceUnchecked = OsmosisPriceSource<String>;
pub type OsmosisPriceSourceChecked = OsmosisPriceSource<Addr>;

impl fmt::Display for OsmosisPriceSourceChecked {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let label = match self {
            OsmosisPriceSource::Fixed {
                price,
            } => format!("fixed:{price}"),
            OsmosisPriceSource::Spot {
                pool_id,
            } => format!("spot:{pool_id}"),
            OsmosisPriceSource::ArithmeticTwap {
                pool_id,
                window_size,
                downtime_detector,
            } => {
                let dd_fmt = DowntimeDetector::fmt(downtime_detector);
                format!("arithmetic_twap:{pool_id}:{window_size}:{dd_fmt}")
            }
            OsmosisPriceSource::GeometricTwap {
                pool_id,
                window_size,
                downtime_detector,
            } => {
                let dd_fmt = DowntimeDetector::fmt(downtime_detector);
                format!("geometric_twap:{pool_id}:{window_size}:{dd_fmt}")
            }
            OsmosisPriceSource::XykLiquidityToken {
                pool_id,
            } => format!("xyk_liquidity_token:{pool_id}"),
            OsmosisPriceSource::StakedGeometricTwap {
                transitive_denom,
                pool_id,
                window_size,
                downtime_detector,
            } => {
                let dd_fmt = DowntimeDetector::fmt(downtime_detector);
                format!("staked_geometric_twap:{transitive_denom}:{pool_id}:{window_size}:{dd_fmt}")
            }
            OsmosisPriceSource::Pyth {
                contract_addr,
                price_feed_id,
                max_staleness,
                denom_decimals,
            } => {
                format!("pyth:{contract_addr}:{price_feed_id}:{max_staleness}:{denom_decimals}")
            }
            OsmosisPriceSource::Lsd {
                transitive_denom,
                geometric_twap,
                redemption_rate,
            } => {
                let GeometricTwap {
                    pool_id,
                    window_size,
                    downtime_detector,
                } = geometric_twap;
                let dd_fmt = DowntimeDetector::fmt(downtime_detector);
                let RedemptionRate {
                    contract_addr,
                    max_staleness,
                } = redemption_rate;
                format!("lsd:{transitive_denom}:{pool_id}:{window_size}:{dd_fmt}:{contract_addr}:{max_staleness}")
            }
        };
        write!(f, "{label}")
    }
}

impl PriceSourceUnchecked<OsmosisPriceSourceChecked, Empty> for OsmosisPriceSourceUnchecked {
    fn validate(
        self,
        deps: &Deps,
        denom: &str,
        base_denom: &str,
        _price_sources: &Map<&str, OsmosisPriceSourceChecked>,
    ) -> ContractResult<OsmosisPriceSourceChecked> {
        match &self {
            OsmosisPriceSourceUnchecked::Fixed {
                price,
            } => Ok(OsmosisPriceSourceChecked::Fixed {
                price: *price,
            }),
            OsmosisPriceSourceUnchecked::Spot {
                pool_id,
            } => {
                let pool = query_pool(&deps.querier, *pool_id)?;
                helpers::assert_osmosis_pool_assets(&pool, denom, base_denom)?;
                Ok(OsmosisPriceSourceChecked::Spot {
                    pool_id: *pool_id,
                })
            }
            OsmosisPriceSourceUnchecked::ArithmeticTwap {
                pool_id,
                window_size,
                downtime_detector,
            } => {
                let pool = query_pool(&deps.querier, *pool_id)?;
                helpers::assert_osmosis_pool_assets(&pool, denom, base_denom)?;
                helpers::assert_osmosis_twap(*window_size, downtime_detector)?;
                Ok(OsmosisPriceSourceChecked::ArithmeticTwap {
                    pool_id: *pool_id,
                    window_size: *window_size,
                    downtime_detector: downtime_detector.clone(),
                })
            }
            OsmosisPriceSourceUnchecked::GeometricTwap {
                pool_id,
                window_size,
                downtime_detector,
            } => {
                let pool = query_pool(&deps.querier, *pool_id)?;
                helpers::assert_osmosis_pool_assets(&pool, denom, base_denom)?;
                helpers::assert_osmosis_twap(*window_size, downtime_detector)?;
                Ok(OsmosisPriceSourceChecked::GeometricTwap {
                    pool_id: *pool_id,
                    window_size: *window_size,
                    downtime_detector: downtime_detector.clone(),
                })
            }
            OsmosisPriceSourceUnchecked::XykLiquidityToken {
                pool_id,
            } => {
                let pool = query_pool(&deps.querier, *pool_id)?;
                helpers::assert_osmosis_xyk_pool(&pool)?;
                Ok(OsmosisPriceSourceChecked::XykLiquidityToken {
                    pool_id: *pool_id,
                })
            }
            OsmosisPriceSourceUnchecked::StakedGeometricTwap {
                transitive_denom,
                pool_id,
                window_size,
                downtime_detector,
            } => {
                let pool = query_pool(&deps.querier, *pool_id)?;
                helpers::assert_osmosis_pool_assets(&pool, denom, transitive_denom)?;
                helpers::assert_osmosis_twap(*window_size, downtime_detector)?;
                Ok(OsmosisPriceSourceChecked::StakedGeometricTwap {
                    transitive_denom: transitive_denom.to_string(),
                    pool_id: *pool_id,
                    window_size: *window_size,
                    downtime_detector: downtime_detector.clone(),
                })
            }
            OsmosisPriceSourceUnchecked::Pyth {
                contract_addr,
                price_feed_id,
                max_staleness,
                denom_decimals,
            } => Ok(OsmosisPriceSourceChecked::Pyth {
                contract_addr: deps.api.addr_validate(contract_addr)?,
                price_feed_id: *price_feed_id,
                max_staleness: *max_staleness,
                denom_decimals: *denom_decimals,
            }),
            OsmosisPriceSourceUnchecked::Lsd {
                transitive_denom,
                geometric_twap,
                redemption_rate,
            } => {
                let pool = query_pool(&deps.querier, geometric_twap.pool_id)?;
                helpers::assert_osmosis_pool_assets(&pool, denom, transitive_denom)?;
                helpers::assert_osmosis_twap(
                    geometric_twap.window_size,
                    &geometric_twap.downtime_detector,
                )?;
                Ok(OsmosisPriceSourceChecked::Lsd {
                    transitive_denom: transitive_denom.to_string(),
                    geometric_twap: geometric_twap.clone(),
                    redemption_rate: RedemptionRate {
                        contract_addr: deps.api.addr_validate(&redemption_rate.contract_addr)?,
                        max_staleness: redemption_rate.max_staleness,
                    },
                })
            }
        }
    }
}

impl PriceSourceChecked<Empty> for OsmosisPriceSourceChecked {
    fn query_price(
        &self,
        deps: &Deps<'_, Empty>,
        env: &Env,
        denom: &str,
        config: &Config,
        price_sources: &Map<&str, Self>,
        kind: ActionKind,
    ) -> ContractResult<Decimal> {
        match self {
            OsmosisPriceSourceChecked::Fixed {
                price,
            } => Ok(*price),
            OsmosisPriceSourceChecked::Spot {
                pool_id,
            } => query_spot_price(&deps.querier, *pool_id, denom, &config.base_denom)
                .map_err(Into::into),
            OsmosisPriceSourceChecked::ArithmeticTwap {
                pool_id,
                window_size,
                downtime_detector,
            } => {
                Self::chain_recovered(deps, downtime_detector)?;

                let start_time = env.block.time.seconds() - window_size;
                query_arithmetic_twap_price(
                    &deps.querier,
                    *pool_id,
                    denom,
                    &config.base_denom,
                    start_time,
                )
                .map_err(Into::into)
            }
            OsmosisPriceSourceChecked::GeometricTwap {
                pool_id,
                window_size,
                downtime_detector,
            } => {
                Self::chain_recovered(deps, downtime_detector)?;

                let start_time = env.block.time.seconds() - window_size;
                query_geometric_twap_price(
                    &deps.querier,
                    *pool_id,
                    denom,
                    &config.base_denom,
                    start_time,
                )
                .map_err(Into::into)
            }
            OsmosisPriceSourceChecked::XykLiquidityToken {
                pool_id,
            } => Self::query_xyk_liquidity_token_price(
                deps,
                env,
                *pool_id,
                config,
                price_sources,
                kind,
            ),
            OsmosisPriceSourceChecked::StakedGeometricTwap {
                transitive_denom,
                pool_id,
                window_size,
                downtime_detector,
            } => {
                Self::chain_recovered(deps, downtime_detector)?;

                Self::query_staked_asset_price(
                    deps,
                    env,
                    denom,
                    transitive_denom,
                    *pool_id,
                    *window_size,
                    config,
                    price_sources,
                    kind,
                )
            }
            OsmosisPriceSourceChecked::Pyth {
                contract_addr,
                price_feed_id,
                max_staleness,
                denom_decimals,
            } => Ok(mars_oracle_base::pyth::query_pyth_price(
                deps,
                env,
                contract_addr.to_owned(),
                *price_feed_id,
                *max_staleness,
                *denom_decimals,
                config,
                price_sources,
                kind,
            )?),
            OsmosisPriceSourceChecked::Lsd {
                transitive_denom,
                geometric_twap,
                redemption_rate,
            } => {
                Self::chain_recovered(deps, &geometric_twap.downtime_detector)?;

                Self::query_lsd_price(
                    deps,
                    env,
                    denom,
                    transitive_denom,
                    geometric_twap.clone(),
                    redemption_rate.clone(),
                    config,
                    price_sources,
                    kind,
                )
            }
        }
    }
}

impl OsmosisPriceSourceChecked {
    fn chain_recovered(
        deps: &Deps,
        downtime_detector: &Option<DowntimeDetector>,
    ) -> ContractResult<()> {
        if let Some(dd) = downtime_detector {
            let recovered = recovered_since_downtime_of_length(
                &deps.querier,
                dd.downtime.clone() as i32,
                dd.recovery,
            )?;
            if !recovered {
                return Err(InvalidPrice {
                    reason: "chain is recovering from downtime".to_string(),
                });
            }
        }

        Ok(())
    }

    /// The calculation of the value of liquidity token, see: https://blog.alphafinance.io/fair-lp-token-pricing/.
    /// This formulation avoids a potential sandwich attack that distorts asset prices by a flashloan.
    ///
    /// NOTE: Price sources must exist for both assets in the pool.
    fn query_xyk_liquidity_token_price(
        deps: &Deps,
        env: &Env,
        pool_id: u64,
        config: &Config,
        price_sources: &Map<&str, Self>,
        kind: ActionKind,
    ) -> ContractResult<Decimal> {
        // XYK pool asserted during price source creation
        let pool = query_pool(&deps.querier, pool_id)?;

        let coin0 = Pool::unwrap_coin(&pool.pool_assets[0].token)?;
        let coin1 = Pool::unwrap_coin(&pool.pool_assets[1].token)?;

        let coin0_price = price_sources.load(deps.storage, &coin0.denom)?.query_price(
            deps,
            env,
            &coin0.denom,
            config,
            price_sources,
            kind.clone(),
        )?;
        let coin1_price = price_sources.load(deps.storage, &coin1.denom)?.query_price(
            deps,
            env,
            &coin1.denom,
            config,
            price_sources,
            kind,
        )?;

        let coin0_value = Uint256::from_uint128(coin0.amount) * Decimal256::from(coin0_price);
        let coin1_value = Uint256::from_uint128(coin1.amount) * Decimal256::from(coin1_price);

        // We need to use Uint256, because Uint128 * Uint128 may overflow the 128-bit limit
        let pool_value_u256 = Uint256::from(2u8) * (coin0_value * coin1_value).isqrt();
        let pool_value_u128 = Uint128::try_from(pool_value_u256)?;

        let total_shares = Pool::unwrap_coin(&pool.total_shares)?.amount;

        Ok(Decimal::from_ratio(pool_value_u128, total_shares))
    }

    /// Staked asset price quoted in OSMO.
    ///
    /// stAsset/OSMO = stAsset/Asset * Asset/OSMO
    /// where:
    /// - stAsset/Asset price calculated using the geometric TWAP from the stAsset/Asset pool.
    /// - Asset/OSMO price comes from the Mars Oracle contract.
    #[allow(clippy::too_many_arguments)]
    fn query_staked_asset_price(
        deps: &Deps,
        env: &Env,
        denom: &str,
        transitive_denom: &str,
        pool_id: u64,
        window_size: u64,
        config: &Config,
        price_sources: &Map<&str, OsmosisPriceSourceChecked>,
        kind: ActionKind,
    ) -> ContractResult<Decimal> {
        let start_time = env.block.time.seconds() - window_size;
        let staked_price = query_geometric_twap_price(
            &deps.querier,
            pool_id,
            denom,
            transitive_denom,
            start_time,
        )?;

        // use current price source
        let transitive_price = price_sources.load(deps.storage, transitive_denom)?.query_price(
            deps,
            env,
            transitive_denom,
            config,
            price_sources,
            kind,
        )?;

        staked_price.checked_mul(transitive_price).map_err(Into::into)
    }

    /// Staked asset price quoted in USD.
    ///
    /// stAsset/USD = stAsset/Asset * Asset/USD
    /// where:
    /// stAsset/Asset = min(stAsset/Asset Geometric TWAP, stAsset/Asset Redemption Rate)
    #[allow(clippy::too_many_arguments)]
    fn query_lsd_price(
        deps: &Deps,
        env: &Env,
        denom: &str,
        transitive_denom: &str,
        geometric_twap: GeometricTwap,
        redemption_rate: RedemptionRate<Addr>,
        config: &Config,
        price_sources: &Map<&str, OsmosisPriceSourceChecked>,
        kind: ActionKind,
    ) -> ContractResult<Decimal> {
        let current_time = env.block.time.seconds();
        let start_time = current_time - geometric_twap.window_size;
        let staked_price = query_geometric_twap_price(
            &deps.querier,
            geometric_twap.pool_id,
            denom,
            transitive_denom,
            start_time,
        )?;

        // query redemption rate
        let rr = query_redemption_rate(
            &deps.querier,
            redemption_rate.contract_addr.clone(),
            denom.to_string(),
            transitive_denom.to_string(),
        )?;
        // Check if the redemption rate is not too old
        if (current_time - rr.last_updated) > redemption_rate.max_staleness {
            return Err(InvalidPrice {
                reason: format!(
                    "redemption rate update time is too old/stale. last updated: {}, now: {}",
                    rr.last_updated, current_time
                ),
            });
        }

        // min from geometric TWAP and exchange rate
        let min_price = min(staked_price, rr.exchange_rate);

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
}
