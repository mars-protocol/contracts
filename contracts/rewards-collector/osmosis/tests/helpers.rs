#![allow(dead_code)]

use std::collections::HashMap;

use cosmwasm_std::{
    coin, from_binary,
    testing::{mock_env, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    Coin, Decimal, Deps, OwnedDeps,
};
use mars_osmosis::helpers::{Pool, QueryPoolResponse};
use mars_red_bank_types::rewards_collector::{Config, InstantiateMsg, QueryMsg};
use mars_rewards_collector_osmosis::contract::entry;
use mars_swapper_osmosis::route::{OsmosisRoute, SwapAmountInRoute};
use mars_testing::{mock_info, MarsMockQuerier};
use osmosis_std::types::osmosis::gamm::v1beta1::PoolAsset;

pub fn mock_instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {
        owner: "owner".to_string(),
        address_provider: "address_provider".to_string(),
        safety_tax_rate: Decimal::percent(25),
        safety_fund_denom: "uusdc".to_string(),
        fee_collector_denom: "umars".to_string(),
        channel_id: "channel-69".to_string(),
        timeout_seconds: 300,
        slippage_tolerance: Decimal::percent(3),
    }
}

pub fn mock_config(api: MockApi, msg: InstantiateMsg) -> Config {
    Config::checked(&api, msg).unwrap()
}

pub fn mock_routes() -> HashMap<(&'static str, &'static str), OsmosisRoute> {
    let mut map = HashMap::new();

    // uosmo -> umars
    map.insert(
        ("uosmo", "umars"),
        OsmosisRoute(vec![SwapAmountInRoute {
            pool_id: 420,
            token_out_denom: "umars".to_string(),
        }]),
    );

    // uatom -> uosmo -> umars
    map.insert(
        ("uatom", "umars"),
        OsmosisRoute(vec![
            SwapAmountInRoute {
                pool_id: 1,
                token_out_denom: "uosmo".to_string(),
            },
            SwapAmountInRoute {
                pool_id: 420,
                token_out_denom: "umars".to_string(),
            },
        ]),
    );

    // uatom -> uosmo -> uusdc
    map.insert(
        ("uatom", "uusdc"),
        OsmosisRoute(vec![
            SwapAmountInRoute {
                pool_id: 1,
                token_out_denom: "uosmo".to_string(),
            },
            SwapAmountInRoute {
                pool_id: 69,
                token_out_denom: "uusdc".to_string(),
            },
        ]),
    );

    // uusdc -> uosmo -> umars
    map.insert(
        ("uusdc", "umars"),
        OsmosisRoute(vec![
            SwapAmountInRoute {
                pool_id: 69,
                token_out_denom: "uosmo".to_string(),
            },
            SwapAmountInRoute {
                pool_id: 420,
                token_out_denom: "umars".to_string(),
            },
        ]),
    );

    map
}

pub fn setup_test() -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
    let mut deps = OwnedDeps::<_, _, _> {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MarsMockQuerier::new(MockQuerier::new(&[(
            MOCK_CONTRACT_ADDR,
            &[coin(88888, "uatom"), coin(1234, "uusdc"), coin(8964, "umars")],
        )])),
        custom_query_type: Default::default(),
    };

    // set up pools for the mock osmosis querier
    deps.querier.set_query_pool_response(
        1,
        prepare_query_pool_response(
            1,
            &[coin(1, "uatom"), coin(1, "uosmo")],
            &[5000u64, 5000u64],
            &coin(1, "uLP"),
        ),
    );
    deps.querier.set_query_pool_response(
        68,
        prepare_query_pool_response(
            68,
            &[coin(1, "uatom"), coin(1, "uusdc")],
            &[5000u64, 5000u64],
            &coin(1, "uLP"),
        ),
    );
    deps.querier.set_query_pool_response(
        69,
        prepare_query_pool_response(
            69,
            &[coin(1, "uosmo"), coin(1, "uusdc")],
            &[5000u64, 5000u64],
            &coin(1, "uLP"),
        ),
    );
    deps.querier.set_query_pool_response(
        420,
        prepare_query_pool_response(
            420,
            &[coin(1, "uosmo"), coin(1, "umars")],
            &[5000u64, 5000u64],
            &coin(1, "uLP"),
        ),
    );

    // instantiate the contract
    let info = mock_info("deployer");
    let msg = mock_instantiate_msg();
    entry::instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps
}

fn prepare_query_pool_response(
    pool_id: u64,
    assets: &[Coin],
    weights: &[u64],
    shares: &Coin,
) -> QueryPoolResponse {
    let pool = Pool {
        address: "address".to_string(),
        id: pool_id.to_string(),
        pool_params: None,
        future_pool_governor: "future_pool_governor".to_string(),
        total_shares: Some(osmosis_std::types::cosmos::base::v1beta1::Coin {
            denom: shares.denom.clone(),
            amount: shares.amount.to_string(),
        }),
        pool_assets: prepare_pool_assets(assets, weights),
        total_weight: "".to_string(),
    };
    QueryPoolResponse {
        pool,
    }
}

fn prepare_pool_assets(coins: &[Coin], weights: &[u64]) -> Vec<PoolAsset> {
    assert_eq!(coins.len(), weights.len());

    coins
        .iter()
        .zip(weights)
        .map(|zipped| {
            let (coin, weight) = zipped;
            PoolAsset {
                token: Some(osmosis_std::types::cosmos::base::v1beta1::Coin {
                    denom: coin.denom.clone(),
                    amount: coin.amount.to_string(),
                }),
                weight: weight.to_string(),
            }
        })
        .collect()
}

pub fn query<T: serde::de::DeserializeOwned>(deps: Deps, msg: QueryMsg) -> T {
    from_binary(&entry::query(deps, mock_env(), msg).unwrap()).unwrap()
}
