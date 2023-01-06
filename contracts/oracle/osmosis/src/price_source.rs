use std::fmt;

use cosmwasm_std::{
    BlockInfo, Decimal, Decimal256, Deps, Empty, Env, Isqrt, QuerierWrapper, Uint128, Uint256,
};
use mars_oracle_base::{ContractError, ContractError::InvalidPrice, ContractResult, PriceSource};
use mars_osmosis::helpers::{
    query_pool, query_spot_price, query_twap_price, recovered_since_downtime_of_length, Pool,
};
use mars_outpost::{oracle, oracle::PriceResponse};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::helpers;

/// 48 hours in seconds
const TWO_DAYS_IN_SECONDS: u64 = 172800u64;

/// Copy from https://github.com/osmosis-labs/osmosis-rust/blob/main/packages/osmosis-std/src/types/osmosis/downtimedetector/v1beta1.rs#L4
/// It doesn't impl JsonSchema.
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

impl fmt::Display for DowntimeDetector {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}:{}", self.downtime, self.recovery)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OsmosisPriceSource {
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
    /// Osmosis twap price quoted in OSMO
    ///
    /// NOTE: `pool_id` must point to an Osmosis pool consists of the asset of interest and OSMO
    Twap {
        pool_id: u64,

        /// Window size in seconds representing the entire window for which 'average' price is calculated.
        /// Value should be <= 172800 sec (48 hours).
        window_size: u64,

        /// Detect when the chain is recovering from downtime
        downtime_detector: Option<DowntimeDetector>,
    },
    /// Osmosis LP token (of an XYK pool) price quoted in OSMO
    XykLiquidityToken {
        pool_id: u64,
    },
}

impl fmt::Display for OsmosisPriceSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let label = match self {
            OsmosisPriceSource::Fixed {
                price,
            } => format!("fixed:{}", price),
            OsmosisPriceSource::Spot {
                pool_id,
            } => format!("spot:{}", pool_id),
            OsmosisPriceSource::Twap {
                pool_id,
                window_size,
                downtime_detector,
            } => {
                let dd_fmt = match downtime_detector {
                    None => "None".to_string(),
                    Some(dd) => format!("Some({})", dd),
                };
                format!("twap:{}:{}:{}", pool_id, window_size, dd_fmt)
            }
            OsmosisPriceSource::XykLiquidityToken {
                pool_id,
            } => format!("xyk_liquidity_token:{}", pool_id),
        };
        write!(f, "{}", label)
    }
}

impl PriceSource<Empty> for OsmosisPriceSource {
    fn validate(
        &self,
        querier: &QuerierWrapper,
        denom: &str,
        base_denom: &str,
    ) -> ContractResult<()> {
        match self {
            OsmosisPriceSource::Fixed {
                ..
            } => Ok(()),
            OsmosisPriceSource::Spot {
                pool_id,
            } => {
                let pool = query_pool(querier, *pool_id)?;
                helpers::assert_osmosis_pool_assets(&pool, denom, base_denom)
            }
            OsmosisPriceSource::Twap {
                pool_id,
                window_size,
                downtime_detector,
            } => {
                let pool = query_pool(querier, *pool_id)?;
                helpers::assert_osmosis_pool_assets(&pool, denom, base_denom)?;

                if *window_size > TWO_DAYS_IN_SECONDS {
                    return Err(ContractError::InvalidPriceSource {
                        reason: format!(
                            "expecting window size to be within {} sec",
                            TWO_DAYS_IN_SECONDS
                        ),
                    });
                }

                if let Some(dd) = downtime_detector {
                    if dd.recovery == 0 {
                        return Err(ContractError::InvalidPriceSource {
                            reason: "downtime recovery can't be 0".to_string(),
                        });
                    }
                }

                Ok(())
            }
            OsmosisPriceSource::XykLiquidityToken {
                pool_id,
            } => {
                let pool = query_pool(querier, *pool_id)?;
                helpers::assert_osmosis_xyk_pool(&pool)
            }
        }
    }

    fn query_price(
        &self,
        deps: &Deps,
        env: &Env,
        denom: &str,
        base_denom: &str,
    ) -> ContractResult<Decimal> {
        match self {
            OsmosisPriceSource::Fixed {
                price,
            } => Ok(*price),
            OsmosisPriceSource::Spot {
                pool_id,
            } => query_spot_price(&deps.querier, *pool_id, denom, base_denom).map_err(Into::into),
            OsmosisPriceSource::Twap {
                pool_id,
                window_size,
                downtime_detector,
            } => Self::query_twap_price(
                deps,
                &env.block,
                denom,
                base_denom,
                *pool_id,
                *window_size,
                downtime_detector,
            ),
            OsmosisPriceSource::XykLiquidityToken {
                pool_id,
            } => Self::query_xyk_liquidity_token_price(deps, env, *pool_id),
        }
    }
}

impl OsmosisPriceSource {
    fn query_twap_price(
        deps: &Deps,
        block: &BlockInfo,
        denom: &str,
        base_denom: &str,
        pool_id: u64,
        window_size: u64,
        downtime_detector: &Option<DowntimeDetector>,
    ) -> ContractResult<Decimal> {
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
        let start_time = block.time.seconds() - window_size;
        query_twap_price(&deps.querier, pool_id, denom, base_denom, start_time).map_err(Into::into)
    }

    /// The calculation of the value of liquidity token, see: https://blog.alphafinance.io/fair-lp-token-pricing/.
    /// This formulation avoids a potential sandwich attack that distorts asset prices by a flashloan.
    ///
    /// NOTE: Price sources must exist for both assets in the pool.
    fn query_xyk_liquidity_token_price(
        deps: &Deps,
        env: &Env,
        pool_id: u64,
    ) -> ContractResult<Decimal> {
        // XYK pool asserted during price source creation
        let pool = query_pool(&deps.querier, pool_id)?;

        let coin0 = Pool::unwrap_coin(&pool.pool_assets[0].token)?;
        let coin1 = Pool::unwrap_coin(&pool.pool_assets[1].token)?;

        let coin0_price_res: PriceResponse = deps.querier.query_wasm_smart(
            env.contract.address.to_string(),
            &oracle::QueryMsg::Price {
                denom: coin0.denom,
            },
        )?;
        let coin1_price_res: PriceResponse = deps.querier.query_wasm_smart(
            env.contract.address.to_string(),
            &oracle::QueryMsg::Price {
                denom: coin1.denom,
            },
        )?;

        let coin0_value =
            Uint256::from_uint128(coin0.amount) * Decimal256::from(coin0_price_res.price);
        let coin1_value =
            Uint256::from_uint128(coin1.amount) * Decimal256::from(coin1_price_res.price);

        // We need to use Uint256, because Uint128 * Uint128 may overflow the 128-bit limit
        let pool_value_u256 = Uint256::from(2u8) * (coin0_value * coin1_value).isqrt();
        let pool_value_u128 = Uint128::try_from(pool_value_u256)?;

        let total_shares = Pool::unwrap_coin(&pool.total_shares)?.amount;

        Ok(Decimal::from_ratio(pool_value_u128, total_shares))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_downtime_detector() {
        let dd = DowntimeDetector {
            downtime: Downtime::Duration10m,
            recovery: 550,
        };
        assert_eq!(dd.to_string(), "Duration10m:550")
    }

    #[test]
    fn display_fixed_price_source() {
        let ps = OsmosisPriceSource::Fixed {
            price: Decimal::from_ratio(1u128, 2u128),
        };
        assert_eq!(ps.to_string(), "fixed:0.5")
    }

    #[test]
    fn display_spot_price_source() {
        let ps = OsmosisPriceSource::Spot {
            pool_id: 123,
        };
        assert_eq!(ps.to_string(), "spot:123")
    }

    #[test]
    fn display_twap_price_source() {
        let ps = OsmosisPriceSource::Twap {
            pool_id: 123,
            window_size: 300,
            downtime_detector: None,
        };
        assert_eq!(ps.to_string(), "twap:123:300:None");

        let ps = OsmosisPriceSource::Twap {
            pool_id: 123,
            window_size: 300,
            downtime_detector: Some(DowntimeDetector {
                downtime: Downtime::Duration30m,
                recovery: 568,
            }),
        };
        assert_eq!(ps.to_string(), "twap:123:300:Some(Duration30m:568)");
    }

    #[test]
    fn display_xyk_lp_price_source() {
        let ps = OsmosisPriceSource::XykLiquidityToken {
            pool_id: 224,
        };
        assert_eq!(ps.to_string(), "xyk_liquidity_token:224")
    }
}
