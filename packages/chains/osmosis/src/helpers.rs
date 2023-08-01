use std::str::FromStr;

use cosmwasm_std::{
    coin, Decimal, Empty, QuerierWrapper, QueryRequest, StdError, StdResult, Uint128,
};
use osmosis_std::{
    shim::{Duration, Timestamp},
    types::{
        cosmos::base::v1beta1::Coin,
        osmosis::{
            downtimedetector::v1beta1::DowntimedetectorQuerier,
            gamm::{
                poolmodels::stableswap::v1beta1::Pool as StableSwapPool,
                v1beta1::Pool as BalancerPool,
            },
            poolmanager::v1beta1::{PoolRequest, PoolResponse, PoolmanagerQuerier},
            twap::v1beta1::TwapQuerier,
        },
    },
};
use prost::Message;

// Get denoms from different type of the pool
pub trait CommonPoolData {
    fn get_pool_id(&self) -> u64;
    fn get_pool_denoms(&self) -> Vec<String>;
}

#[derive(Debug, PartialEq)]
pub enum Pool {
    Balancer(BalancerPool),
    StableSwap(StableSwapPool),
}

impl CommonPoolData for Pool {
    fn get_pool_id(&self) -> u64 {
        match self {
            Pool::Balancer(pool) => pool.id,
            Pool::StableSwap(pool) => pool.id,
        }
    }

    fn get_pool_denoms(&self) -> Vec<String> {
        match self {
            Pool::Balancer(pool) => pool
                .pool_assets
                .iter()
                .flat_map(|asset| &asset.token)
                .map(|token| token.denom.clone())
                .collect(),
            Pool::StableSwap(pool) => {
                pool.pool_liquidity.iter().map(|pl| pl.denom.clone()).collect()
            }
        }
    }
}

impl TryFrom<osmosis_std::shim::Any> for Pool {
    type Error = StdError;

    fn try_from(value: osmosis_std::shim::Any) -> Result<Self, Self::Error> {
        if let Ok(pool) = BalancerPool::decode(value.value.as_slice()) {
            return Ok(Pool::Balancer(pool));
        }
        if let Ok(pool) = StableSwapPool::decode(value.value.as_slice()) {
            return Ok(Pool::StableSwap(pool));
        }

        Err(StdError::parse_err(
            "Pool",
            "Unsupported pool: must be either `Balancer` or `StableSwap`.",
        ))
    }
}

impl Pool {
    /// Unwraps Osmosis coin into Cosmwasm coin
    pub fn unwrap_coin(osmosis_coin: &Option<Coin>) -> StdResult<cosmwasm_std::Coin> {
        let osmosis_coin = match osmosis_coin {
            None => return Err(StdError::generic_err("missing coin")), // just in case, it shouldn't happen
            Some(osmosis_coin) => osmosis_coin,
        };
        let cosmwasm_coin =
            coin(Uint128::from_str(&osmosis_coin.amount)?.u128(), &osmosis_coin.denom);
        Ok(cosmwasm_coin)
    }
}

/// Query an Osmosis pool's coin depths and the supply of of liquidity token
pub fn query_pool(querier: &QuerierWrapper, pool_id: u64) -> StdResult<Pool> {
    let req: QueryRequest<Empty> = PoolRequest {
        pool_id,
    }
    .into();
    let res: PoolResponse = querier.query(&req)?;
    res.pool.ok_or_else(|| StdError::not_found("pool"))?.try_into() // convert `Any` to `Pool`
}

/// Query the spot price of a coin, denominated in OSMO
pub fn query_spot_price(
    querier: &QuerierWrapper,
    pool_id: u64,
    base_denom: &str,
    quote_denom: &str,
) -> StdResult<Decimal> {
    let spot_price_res = PoolmanagerQuerier::new(querier).spot_price(
        pool_id,
        base_denom.to_string(),
        quote_denom.to_string(),
    )?;
    let price = Decimal::from_str(&spot_price_res.spot_price)?;
    Ok(price)
}

/// Query arithmetic twap price of a coin, denominated in OSMO.
/// `start_time` must be within 48 hours of current block time.
pub fn query_arithmetic_twap_price(
    querier: &QuerierWrapper,
    pool_id: u64,
    base_denom: &str,
    quote_denom: &str,
    start_time: u64,
) -> StdResult<Decimal> {
    let twap_res = TwapQuerier::new(querier).arithmetic_twap_to_now(
        pool_id,
        base_denom.to_string(),
        quote_denom.to_string(),
        Some(Timestamp {
            seconds: start_time as i64,
            nanos: 0,
        }),
    )?;
    let price = Decimal::from_str(&twap_res.arithmetic_twap)?;
    Ok(price)
}

/// Query geometric twap price of a coin, denominated in OSMO.
/// `start_time` must be within 48 hours of current block time.
pub fn query_geometric_twap_price(
    querier: &QuerierWrapper,
    pool_id: u64,
    base_denom: &str,
    quote_denom: &str,
    start_time: u64,
) -> StdResult<Decimal> {
    let twap_res = TwapQuerier::new(querier).geometric_twap_to_now(
        pool_id,
        base_denom.to_string(),
        quote_denom.to_string(),
        Some(Timestamp {
            seconds: start_time as i64,
            nanos: 0,
        }),
    )?;
    let price = Decimal::from_str(&twap_res.geometric_twap)?;
    Ok(price)
}

/// Has it been $RECOVERY_PERIOD since the chain has been down for $DOWNTIME_PERIOD.
///
/// https://github.com/osmosis-labs/osmosis/tree/main/x/downtime-detector
pub fn recovered_since_downtime_of_length(
    querier: &QuerierWrapper,
    downtime: i32,
    recovery: u64,
) -> StdResult<bool> {
    let downtime_detector_res = DowntimedetectorQuerier::new(querier)
        .recovered_since_downtime_of_length(
            downtime,
            Some(Duration {
                seconds: recovery as i64,
                nanos: 0,
            }),
        )?;
    Ok(downtime_detector_res.succesfully_recovered)
}

#[cfg(test)]
mod tests {
    use osmosis_std::types::osmosis::gamm::v1beta1::PoolAsset;

    use super::*;

    #[test]
    fn unwrapping_coin() {
        let pool = BalancerPool {
            id: 1111,
            address: "".to_string(),
            pool_params: None,
            future_pool_governor: "".to_string(),
            pool_assets: vec![
                PoolAsset {
                    token: Some(Coin {
                        denom: "denom_1".to_string(),
                        amount: "123".to_string(),
                    }),
                    weight: "500".to_string(),
                },
                PoolAsset {
                    token: Some(Coin {
                        denom: "denom_2".to_string(),
                        amount: "430".to_string(),
                    }),
                    weight: "500".to_string(),
                },
            ],
            total_shares: None,
            total_weight: "".to_string(),
        };

        let res_err = Pool::unwrap_coin(&pool.total_shares).unwrap_err();
        assert_eq!(res_err, StdError::generic_err("missing coin"));

        let res = Pool::unwrap_coin(&pool.pool_assets[0].token).unwrap();
        assert_eq!(res, coin(123, "denom_1"));
        let res = Pool::unwrap_coin(&pool.pool_assets[1].token).unwrap();
        assert_eq!(res, coin(430, "denom_2"));
    }

    #[test]
    fn common_data_for_balancer_pool() {
        let balancer_pool = BalancerPool {
            id: 1111,
            address: "".to_string(),
            pool_params: None,
            future_pool_governor: "".to_string(),
            pool_assets: vec![
                PoolAsset {
                    token: Some(Coin {
                        denom: "denom_1".to_string(),
                        amount: "123".to_string(),
                    }),
                    weight: "500".to_string(),
                },
                PoolAsset {
                    token: Some(Coin {
                        denom: "denom_2".to_string(),
                        amount: "430".to_string(),
                    }),
                    weight: "500".to_string(),
                },
            ],
            total_shares: None,
            total_weight: "".to_string(),
        };

        let any_pool = balancer_pool.to_any();
        let pool: Pool = any_pool.try_into().unwrap();

        assert_eq!(balancer_pool.id, pool.get_pool_id());
        assert_eq!(vec!["denom_1".to_string(), "denom_2".to_string()], pool.get_pool_denoms())
    }

    #[test]
    fn common_data_for_stable_swap_pool() {
        let stable_swap_pool = StableSwapPool {
            address: "".to_string(),
            id: 4444,
            pool_params: None,
            future_pool_governor: "".to_string(),
            total_shares: None,
            pool_liquidity: vec![
                Coin {
                    denom: "denom_1".to_string(),
                    amount: "123".to_string(),
                },
                Coin {
                    denom: "denom_2".to_string(),
                    amount: "430".to_string(),
                },
            ],
            scaling_factors: vec![],
            scaling_factor_controller: "".to_string(),
        };

        let any_pool = stable_swap_pool.to_any();
        let pool: Pool = any_pool.try_into().unwrap();

        assert_eq!(stable_swap_pool.id, pool.get_pool_id());
        assert_eq!(vec!["denom_1".to_string(), "denom_2".to_string()], pool.get_pool_denoms())
    }
}
