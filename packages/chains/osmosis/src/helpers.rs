use std::str::FromStr;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    coin, from_json, Decimal, Empty, QuerierWrapper, QueryRequest, StdError, StdResult, Uint128,
};
use osmosis_std::{
    shim::{Duration, Timestamp},
    types::{
        cosmos::base::v1beta1::Coin,
        osmosis::{
            concentratedliquidity::v1beta1::Pool as ConcentratedLiquidityPool,
            cosmwasmpool::v1beta1::{CosmWasmPool as OsmoCosmWasmPool, InstantiateMsg},
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

#[derive(Debug, PartialEq)]
pub struct CosmWasmPool {
    pub id: u64,
    pub pool_asset_configs: Vec<AssetConfig>,
}

impl CosmWasmPool {
    pub fn query_price(&self, denom_in: &str, denom_out: &str) -> StdResult<Decimal> {
        if denom_in == denom_out {
            return Err(StdError::generic_err("denom_in and denom_out must be different"));
        }

        let asset_in =
            self.pool_asset_configs.iter().find(|ac| ac.denom == denom_in).ok_or_else(|| {
                StdError::generic_err(format!("asset config for {} not found", denom_in))
            })?;
        let asset_out =
            self.pool_asset_configs.iter().find(|ac| ac.denom == denom_out).ok_or_else(|| {
                StdError::generic_err(format!("asset config for {} not found", denom_out))
            })?;

        // denom_out_amount / denom_out_normalization_factor = denom_in_amount / denom_in_normalization_factor
        // denom_out_amount = denom_in_amount * denom_out_normalization_factor / denom_in_normalization_factor
        // For more details, see: https://github.com/osmosis-labs/transmuter/blob/main/contracts/transmuter/src/asset.rs#L174
        Ok(Decimal::from_ratio(
            asset_out.normalization_factor.u128(),
            asset_in.normalization_factor.u128(),
        ))
    }
}

/// Fields taken from Instantiate msg https://github.com/osmosis-labs/transmuter/blob/47bbb023463578937a7086ad80071196126349d9/contracts/transmuter/src/contract.rs#L74
#[cw_serde]
struct TransmuterV3InstantiateMsg {
    pub pool_asset_configs: Vec<AssetConfig>,
    pub alloyed_asset_subdenom: String,
    pub alloyed_asset_normalization_factor: Uint128,
    pub admin: Option<String>,
    pub moderator: Option<String>,
}

#[cw_serde]
pub struct AssetConfig {
    pub denom: String,
    pub normalization_factor: Uint128,
}

// Get denoms from different type of the pool
pub trait CommonPoolData {
    fn get_pool_id(&self) -> u64;
    fn get_pool_denoms(&self) -> Vec<String>;
}

#[derive(Debug, PartialEq)]
pub enum Pool {
    Balancer(BalancerPool),
    StableSwap(StableSwapPool),
    ConcentratedLiquidity(ConcentratedLiquidityPool),
    CosmWasm(CosmWasmPool),
}

impl CommonPoolData for Pool {
    fn get_pool_id(&self) -> u64 {
        match self {
            Pool::Balancer(pool) => pool.id,
            Pool::StableSwap(pool) => pool.id,
            Pool::ConcentratedLiquidity(pool) => pool.id,
            Pool::CosmWasm(pool) => pool.id,
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
            Pool::ConcentratedLiquidity(pool) => {
                vec![pool.token0.clone(), pool.token1.clone()]
            }
            Pool::CosmWasm(pool) => {
                pool.pool_asset_configs.iter().map(|ac| ac.denom.clone()).collect()
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

        if let Ok(pool) = ConcentratedLiquidityPool::decode(value.value.as_slice()) {
            return Ok(Pool::ConcentratedLiquidity(pool));
        }

        if let Ok(pool) = OsmoCosmWasmPool::decode(value.value.as_slice()) {
            // Try to parse the instantiate message of the cosmwasm pool:
            // V1:
            // ```json
            // {
            //  "pool_asset_denoms": [
            //      "ibc/40F1B2458AEDA66431F9D44F48413240B8D28C072463E2BF53655728683583E3",
            //      "ibc/6F34E1BD664C36CE49ACC28E60D62559A5F96C4F9A6CCE4FC5A67B2852E24CFE"
            //  ]
            // }
            //
            // V2:
            // ```json
            // {
            //  "pool_asset_denoms": [
            //      "uosmo",
            //      "factory/osmo14eq94mckd6kp0pwnxx33ycpk762z7rum29epr3/teko02"
            //  ],
            //  "admin": "osmo14eq94mckd6kp0pwnxx33ycpk762z7rum29epr3",
            //  "alloyed_asset_subdenom": "teko"
            // }
            //
            // Both of them have the same field `pool_asset_denoms` and it's the only field we need to use.
            if let Ok(msg) = from_json::<InstantiateMsg>(&pool.instantiate_msg) {
                return Ok(Pool::CosmWasm(CosmWasmPool {
                    id: pool.pool_id,
                    pool_asset_configs: msg
                        .pool_asset_denoms
                        .iter()
                        .map(|denom| AssetConfig {
                            denom: denom.clone(),
                            normalization_factor: Uint128::one(), // 1:1 conversion of one asset to another
                        })
                        .collect(),
                }));
            }

            // try to parse the instantiate message V3
            if let Ok(msg) = from_json::<TransmuterV3InstantiateMsg>(&pool.instantiate_msg) {
                return Ok(Pool::CosmWasm(CosmWasmPool {
                    id: pool.pool_id,
                    pool_asset_configs: msg.pool_asset_configs,
                }));
            }

            return Err(StdError::parse_err(
                "Pool",
                "Failed to parse CosmWasm pool instantiate message.",
            ));
        }

        Err(StdError::parse_err(
            "Pool",
            "Unsupported pool: must be either `Balancer`, `StableSwap`, `ConcentratedLiquidity` or CosmWasm transmuter.",
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
    use cosmwasm_std::to_json_vec;
    use osmosis_std::types::osmosis::gamm::v1beta1::PoolAsset;

    use super::*;

    #[cw_serde]
    struct TransmuterV1InstantiateMsg {
        pub pool_asset_denoms: Vec<String>,
        pub alloyed_asset_subdenom: String,
        pub admin: Option<String>,
    }

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

    #[test]
    fn common_data_for_concentrated_liquidity_pool() {
        let concentrated_liquidity_pool = ConcentratedLiquidityPool {
            address: "pool_address".to_string(),
            incentives_address: "incentives_address".to_string(),
            spread_rewards_address: "spread_rewards_address".to_string(),
            id: 1066,
            current_tick_liquidity: "3820025893854099618.699762490947860933".to_string(),
            token0: "uosmo".to_string(),
            token1: "ibc/0CD3A0285E1341859B5E86B6AB7682F023D03E97607CCC1DC95706411D866DF7"
                .to_string(),
            current_sqrt_price: "656651.537483144215151633465586753226461989".to_string(),
            current_tick: 102311912,
            tick_spacing: 100,
            exponent_at_price_one: -6,
            spread_factor: "0.002000000000000000".to_string(),
            last_liquidity_update: None,
        };

        let any_pool = concentrated_liquidity_pool.to_any();
        let pool: Pool = any_pool.try_into().unwrap();

        assert_eq!(concentrated_liquidity_pool.id, pool.get_pool_id());
        assert_eq!(
            vec![
                "uosmo".to_string(),
                "ibc/0CD3A0285E1341859B5E86B6AB7682F023D03E97607CCC1DC95706411D866DF7".to_string()
            ],
            pool.get_pool_denoms()
        );
    }

    #[test]
    fn common_data_for_cosmwasm_pool_v1() {
        let msg = InstantiateMsg {
            pool_asset_denoms: vec![
                "ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858".to_string(),
                "ibc/498A0751C798A0D9A389AA3691123DADA57DAA4FE165D5C75894505B876BA6E4".to_string(),
            ],
        };
        let cosmwasm_pool = OsmoCosmWasmPool {
            contract_address: "pool_address".to_string(),
            pool_id: 1212,
            code_id: 148,
            instantiate_msg: to_json_vec(&msg).unwrap(),
        };

        let any_pool = cosmwasm_pool.to_any();
        let pool: Pool = any_pool.try_into().unwrap();

        assert_eq!(cosmwasm_pool.pool_id, pool.get_pool_id());
        assert_eq!(
            vec![
                "ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858".to_string(),
                "ibc/498A0751C798A0D9A389AA3691123DADA57DAA4FE165D5C75894505B876BA6E4".to_string()
            ],
            pool.get_pool_denoms()
        );
    }

    #[test]
    fn common_data_for_cosmwasm_pool_v2() {
        // check if extra fields are ignored during deserialization
        let msg = TransmuterV1InstantiateMsg {
            pool_asset_denoms: vec![
                "uosmo".to_string(),
                "factory/osmo14eq94mckd6kp0pwnxx33ycpk762z7rum29epr3/teko02".to_string(),
            ],
            alloyed_asset_subdenom: "teko".to_string(),
            admin: Some("osmo14eq94mckd6kp0pwnxx33ycpk762z7rum29epr3".to_string()),
        };
        let cosmwasm_pool = OsmoCosmWasmPool {
            contract_address: "pool_address".to_string(),
            pool_id: 1212,
            code_id: 148,
            instantiate_msg: to_json_vec(&msg).unwrap(),
        };

        let any_pool = cosmwasm_pool.to_any();
        let pool: Pool = any_pool.try_into().unwrap();

        assert_eq!(cosmwasm_pool.pool_id, pool.get_pool_id());
        assert_eq!(
            vec![
                "uosmo".to_string(),
                "factory/osmo14eq94mckd6kp0pwnxx33ycpk762z7rum29epr3/teko02".to_string(),
            ],
            pool.get_pool_denoms()
        );
    }

    #[test]
    fn common_data_for_cosmwasm_pool_v3() {
        let msg = InstantiateMsg {
            pool_asset_denoms: vec![
                "ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858".to_string(),
                "ibc/498A0751C798A0D9A389AA3691123DADA57DAA4FE165D5C75894505B876BA6E4".to_string(),
            ],
        };
        let cosmwasm_pool = OsmoCosmWasmPool {
            contract_address: "pool_address".to_string(),
            pool_id: 1212,
            code_id: 148,
            instantiate_msg: to_json_vec(&msg).unwrap(),
        };

        let any_pool = cosmwasm_pool.to_any();
        let pool: Pool = any_pool.try_into().unwrap();

        assert_eq!(cosmwasm_pool.pool_id, pool.get_pool_id());
        assert_eq!(
            vec![
                "ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858".to_string(),
                "ibc/498A0751C798A0D9A389AA3691123DADA57DAA4FE165D5C75894505B876BA6E4".to_string()
            ],
            pool.get_pool_denoms()
        );
    }

    #[test]
    fn cosmwasm_pool_error_handled() {
        let msg = InstantiateMsg {
            pool_asset_denoms: vec![
                "ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858".to_string(),
                "ibc/498A0751C798A0D9A389AA3691123DADA57DAA4FE165D5C75894505B876BA6E4".to_string(),
            ],
        };
        let cosmwasm_pool = OsmoCosmWasmPool {
            contract_address: "pool_address".to_string(),
            pool_id: 1212,
            code_id: 148,
            instantiate_msg: msg.encode_to_vec(),
        };

        let any_pool = cosmwasm_pool.to_any();
        let pool: Result<Pool, StdError> = any_pool.try_into();

        assert_eq!(
            pool.unwrap_err(),
            StdError::parse_err("Pool", "Failed to parse CosmWasm pool instantiate message.",)
        );
    }

    #[test]
    fn cosmwasm_pool_price_error_if_denoms_the_same() {
        let cw_pool = CosmWasmPool {
            id: 1212,
            pool_asset_configs: vec![
                AssetConfig {
                    denom: "nobleUsdc".to_string(),
                    normalization_factor: Uint128::one(),
                },
                AssetConfig {
                    denom: "axlUsdc".to_string(),
                    normalization_factor: Uint128::one(),
                },
            ],
        };

        let res = cw_pool.query_price("nobleUsdc", "nobleUsdc");
        assert_eq!(
            res.unwrap_err(),
            StdError::generic_err("denom_in and denom_out must be different")
        );
    }

    #[test]
    fn cosmwasm_pool_price_error_if_missing_denom() {
        let cw_pool = CosmWasmPool {
            id: 1212,
            pool_asset_configs: vec![
                AssetConfig {
                    denom: "nobleUsdc".to_string(),
                    normalization_factor: Uint128::one(),
                },
                AssetConfig {
                    denom: "axlUsdc".to_string(),
                    normalization_factor: Uint128::one(),
                },
            ],
        };

        let res = cw_pool.query_price("nobleUsdc", "uosmo");
        assert_eq!(res.unwrap_err(), StdError::generic_err("asset config for uosmo not found"));

        let res = cw_pool.query_price("untrn", "nobleUsdc");
        assert_eq!(res.unwrap_err(), StdError::generic_err("asset config for untrn not found"));
    }

    #[test]
    fn cosmwasm_pool_price() {
        let cw_pool = CosmWasmPool {
            id: 1212,
            pool_asset_configs: vec![
                AssetConfig {
                    denom: "nobleUsdc".to_string(),
                    normalization_factor: Uint128::new(1000),
                },
                AssetConfig {
                    denom: "axlUsdc".to_string(),
                    normalization_factor: Uint128::new(200),
                },
            ],
        };

        let price = cw_pool.query_price("axlUsdc", "nobleUsdc").unwrap();
        assert_eq!(price, Decimal::from_ratio(1000u128, 200u128));

        let price = cw_pool.query_price("nobleUsdc", "axlUsdc").unwrap();
        assert_eq!(price, Decimal::from_ratio(200u128, 1000u128));
    }
}
