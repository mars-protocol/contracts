use std::fmt;

use cosmwasm_std::{Decimal, Env, QuerierWrapper, StdError, StdResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use mars_oracle_base::{ContractError, ContractResult, PriceSource};

use osmo_bindings::OsmosisQuery;

use crate::helpers;

/// 48 hours in seconds
const TWO_DAYS_IN_SECONDS: u64 = 172800u64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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
    },
    /// Osmosis liquidity token
    LiquidityToken {
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
                ..
            } => format!("twap:{}", pool_id),
            OsmosisPriceSource::LiquidityToken {
                pool_id,
            } => format!("liquidity_token:{}", pool_id),
        };
        write!(f, "{}", label)
    }
}

impl PriceSource<OsmosisQuery> for OsmosisPriceSource {
    fn validate(
        &self,
        querier: &QuerierWrapper<OsmosisQuery>,
        denom: &str,
        base_denom: &str,
    ) -> ContractResult<()> {
        match self {
            OsmosisPriceSource::Fixed {
                ..
            } => Ok(()),
            OsmosisPriceSource::Spot {
                pool_id,
            } => helpers::assert_osmosis_pool_assets(querier, *pool_id, denom, base_denom),
            OsmosisPriceSource::Twap {
                pool_id,
                window_size,
            } => {
                helpers::assert_osmosis_pool_assets(querier, *pool_id, denom, base_denom)?;

                if *window_size > TWO_DAYS_IN_SECONDS {
                    Err(ContractError::InvalidPriceSource {
                        reason: format!(
                            "expecting window size to be within {} sec",
                            TWO_DAYS_IN_SECONDS
                        ),
                    })
                } else {
                    Ok(())
                }
            }
            OsmosisPriceSource::LiquidityToken {
                ..
            } => Ok(()),
        }
    }

    fn query_price(
        &self,
        querier: &QuerierWrapper<OsmosisQuery>,
        env: Env,
        denom: &str,
        base_denom: &str,
    ) -> StdResult<Decimal> {
        match self {
            OsmosisPriceSource::Fixed {
                price,
            } => Ok(*price),
            OsmosisPriceSource::Spot {
                pool_id,
            } => helpers::query_osmosis_spot_price(querier, *pool_id, denom, base_denom),
            OsmosisPriceSource::Twap {
                pool_id,
                window_size,
            } => {
                let current_block_time = env.block.time.seconds();
                let start_time = current_block_time - window_size;
                helpers::query_osmosis_twap_price(querier, *pool_id, denom, base_denom, start_time)
            }
            OsmosisPriceSource::LiquidityToken {
                ..
            } => Err(StdError::generic_err("Unimplemented")),
        }
    }
}
