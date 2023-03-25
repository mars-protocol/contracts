#![allow(dead_code)]

use std::marker::PhantomData;

use cosmwasm_std::{
    coin, from_binary,
    testing::{mock_env, MockApi, MockQuerier, MockStorage},
    Coin, Deps, DepsMut, OwnedDeps,
};
use mars_oracle_base::ContractError;
use mars_oracle_osmosis::{contract::entry, msg::ExecuteMsg, OsmosisPriceSource};
use mars_osmosis::helpers::{Pool, QueryPoolResponse};
use mars_red_bank_types::oracle::{InstantiateMsg, QueryMsg};
use mars_testing::{mock_info, MarsMockQuerier};
use osmosis_std::types::osmosis::gamm::v1beta1::PoolAsset;

pub fn setup_test() -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
    let mut deps = OwnedDeps::<_, _, _> {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MarsMockQuerier::new(MockQuerier::new(&[])),
        custom_query_type: PhantomData,
    };

    // set a few osmosis pools
    let assets = vec![coin(42069, "uatom"), coin(69420, "uosmo")];
    deps.querier.set_query_pool_response(
        1,
        prepare_query_pool_response(1, &assets, &[5000u64, 5000u64], &coin(10000, "gamm/pool/1")),
    );

    let assets = vec![coin(12345, "uusdc"), coin(23456, "uatom")];
    deps.querier.set_query_pool_response(
        64,
        prepare_query_pool_response(64, &assets, &[5000u64, 5000u64], &coin(10000, "gamm/pool/64")),
    );

    let assets = vec![coin(12345, "uosmo"), coin(88888, "umars")];
    deps.querier.set_query_pool_response(
        89,
        prepare_query_pool_response(89, &assets, &[5000u64, 5000u64], &coin(10000, "gamm/pool/89")),
    );

    let assets = vec![coin(12345, "ustatom"), coin(88888, "uatom")];
    deps.querier.set_query_pool_response(
        803,
        prepare_query_pool_response(
            803,
            &assets,
            &[5000u64, 5000u64],
            &coin(10000, "gamm/pool/803"),
        ),
    );

    let assets = vec![coin(100000, "uusdc"), coin(100000, "uusdt"), coin(100000, "udai")];
    deps.querier.set_query_pool_response(
        3333,
        prepare_query_pool_response(
            3333,
            &assets,
            &[5000u64, 5000u64, 5000u64],
            &coin(10000, "gamm/pool/3333"),
        ),
    );

    // Set not XYK pool (different assets weights)
    let assets = vec![coin(100000, "uion"), coin(100000, "uosmo")];
    deps.querier.set_query_pool_response(
        4444,
        prepare_query_pool_response(
            4444,
            &assets,
            &[5000u64, 5005u64],
            &coin(10000, "gamm/pool/4444"),
        ),
    );

    // instantiate the oracle contract
    entry::instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        InstantiateMsg {
            owner: "owner".to_string(),
            base_denom: "uosmo".to_string(),
        },
    )
    .unwrap();

    deps
}

pub fn prepare_query_pool_response(
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

pub fn set_price_source(deps: DepsMut, denom: &str, price_source: OsmosisPriceSource) {
    entry::execute(
        deps,
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: denom.to_string(),
            price_source,
        },
    )
    .unwrap();
}

pub fn query<T: serde::de::DeserializeOwned>(deps: Deps, msg: QueryMsg) -> T {
    from_binary(&entry::query(deps, mock_env(), msg).unwrap()).unwrap()
}

pub fn query_err(deps: Deps, msg: QueryMsg) -> ContractError {
    entry::query(deps, mock_env(), msg).unwrap_err()
}
