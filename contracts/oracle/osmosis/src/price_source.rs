use std::fmt;

use cosmwasm_std::{
    Decimal, Decimal256, Deps, Empty, Env, Isqrt, QuerierWrapper, StdResult, Uint128, Uint256,
};
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use mars_oracle_base::{ContractError, ContractResult, PriceSource};
use mars_osmosis::helpers::{query_pool, query_spot_price, query_twap_price, Pool};

use crate::helpers;

/// 48 hours in seconds
const TWO_DAYS_IN_SECONDS: u64 = 172800u64;

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
            } => format!("twap:{}:{}", pool_id, window_size),
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
            } => {
                let pool = query_pool(querier, *pool_id)?;
                helpers::assert_osmosis_pool_assets(&pool, denom, base_denom)?;

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
        price_sources: &Map<&str, Self>,
    ) -> StdResult<Decimal> {
        match self {
            OsmosisPriceSource::Fixed {
                price,
            } => Ok(*price),
            OsmosisPriceSource::Spot {
                pool_id,
            } => query_spot_price(&deps.querier, *pool_id, denom, base_denom),
            OsmosisPriceSource::Twap {
                pool_id,
                window_size,
            } => {
                let start_time = env.block.time.seconds() - window_size;
                query_twap_price(&deps.querier, *pool_id, denom, base_denom, start_time)
            }
            OsmosisPriceSource::XykLiquidityToken {
                pool_id,
            } => {
                self.query_xyk_liquidity_token_price(deps, env, *pool_id, base_denom, price_sources)
            }
        }
    }
}

impl OsmosisPriceSource {
    /// The calculation of the value of liquidity token, see: https://blog.alphafinance.io/fair-lp-token-pricing/.
    /// This formulation avoids a potential sandwich attack that distorts asset prices by a flashloan.
    ///
    /// NOTE: Price sources must exist for both assets in the pool.
    fn query_xyk_liquidity_token_price(
        &self,
        deps: &Deps,
        env: &Env,
        pool_id: u64,
        base_denom: &str,
        price_sources: &Map<&str, Self>,
    ) -> StdResult<Decimal> {
        // XYK pool asserted during price source creation
        let pool = query_pool(&deps.querier, pool_id)?;

        let coin0 = Pool::unwrap_coin(&pool.pool_assets[0].token)?;
        let coin1 = Pool::unwrap_coin(&pool.pool_assets[1].token)?;

        let coin0_price = price_sources.load(deps.storage, &coin0.denom)?.query_price(
            deps,
            env,
            &coin0.denom,
            base_denom,
            price_sources,
        )?;
        let coin1_price = price_sources.load(deps.storage, &coin1.denom)?.query_price(
            deps,
            env,
            &coin1.denom,
            base_denom,
            price_sources,
        )?;

        let coin0_value = Uint256::from_uint128(coin0.amount) * Decimal256::from(coin0_price);
        let coin1_value = Uint256::from_uint128(coin1.amount) * Decimal256::from(coin1_price);

        // We need to use Uint256, because Uint128 * Uint128 may overflow the 128-bit limit
        let pool_value_u256 = Uint256::from(2u8) * (coin0_value * coin1_value).isqrt();
        let pool_value_u128 = Uint128::try_from(pool_value_u256)?;

        let total_shares = Pool::unwrap_coin(&pool.total_shares)?.amount;

        Ok(Decimal::from_ratio(pool_value_u128, total_shares))
    }
}
