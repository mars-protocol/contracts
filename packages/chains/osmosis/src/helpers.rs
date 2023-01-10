use std::str::FromStr;

use cosmwasm_std::{
    coin, Decimal, Empty, QuerierWrapper, QueryRequest, StdError, StdResult, Uint128,
};
use osmosis_std::{
    shim::Timestamp,
    types::{
        cosmos::base::v1beta1::Coin,
        osmosis::{
            gamm::{
                v1beta1::{PoolAsset, PoolParams, QueryPoolRequest},
                v2::GammQuerier,
            },
            twap::v1beta1::TwapQuerier,
        },
    },
};
use serde::{Deserialize, Serialize};

// NOTE: Use custom Pool (`id` type as String) due to problem with json (de)serialization discrepancy between go and rust side.
// https://github.com/osmosis-labs/osmosis-rust/issues/42
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Pool {
    pub id: String,
    pub address: String,
    pub pool_params: Option<PoolParams>,
    pub future_pool_governor: String,
    pub pool_assets: Vec<PoolAsset>,
    pub total_shares: Option<Coin>,
    pub total_weight: String,
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct QueryPoolResponse {
    pub pool: Pool,
}

/// Query an Osmosis pool's coin depths and the supply of of liquidity token
pub fn query_pool(querier: &QuerierWrapper, pool_id: u64) -> StdResult<Pool> {
    let req: QueryRequest<Empty> = QueryPoolRequest {
        pool_id,
    }
    .into();
    let res: QueryPoolResponse = querier.query(&req)?;
    Ok(res.pool)
}

pub fn has_denom(denom: &str, pool_assets: &[PoolAsset]) -> bool {
    pool_assets.iter().flat_map(|asset| &asset.token).any(|coin| coin.denom == denom)
}

/// Query the spot price of a coin, denominated in OSMO
pub fn query_spot_price(
    querier: &QuerierWrapper,
    pool_id: u64,
    base_denom: &str,
    quote_denom: &str,
) -> StdResult<Decimal> {
    let spot_price_res = GammQuerier::new(querier).spot_price(
        pool_id,
        base_denom.to_string(),
        quote_denom.to_string(),
    )?;
    let price = Decimal::from_str(&spot_price_res.spot_price)?;
    Ok(price)
}

/// Query the twap price of a coin, denominated in OSMO.
/// `start_time` must be within 48 hours of current block time.
#[allow(deprecated)] // FIXME: arithmetic_twap_to_now shouldn't be deprecated, make clippy happy for now
pub fn query_twap_price(
    querier: &QuerierWrapper,
    pool_id: u64,
    base_denom: &str,
    quote_denom: &str,
    start_time: u64,
) -> StdResult<Decimal> {
    let arithmetic_twap_res = TwapQuerier::new(querier).arithmetic_twap_to_now(
        pool_id,
        base_denom.to_string(),
        quote_denom.to_string(),
        Some(Timestamp {
            seconds: start_time as i64,
            nanos: 0,
        }),
    )?;
    let price = Decimal::from_str(&arithmetic_twap_res.arithmetic_twap)?;
    Ok(price)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unwrapping_coin() {
        let pool = Pool {
            id: "1111".to_string(),
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
}
