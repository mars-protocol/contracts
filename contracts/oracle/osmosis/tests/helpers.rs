#![allow(dead_code)]

use std::{marker::PhantomData, str::FromStr};

use cosmwasm_std::{
    coin, from_binary,
    testing::{mock_env, MockApi, MockQuerier, MockStorage},
    Coin, Decimal, Deps, DepsMut, OwnedDeps,
};
use mars_oracle_base::ContractError;
use mars_oracle_osmosis::{contract::entry, msg::ExecuteMsg, OsmosisPriceSourceUnchecked};
use mars_osmosis::{BalancerPool, StableSwapPool};
use mars_red_bank_types::oracle::msg::{InstantiateMsg, QueryMsg};
use mars_testing::{mock_info, MarsMockQuerier};
use osmosis_std::types::osmosis::{gamm::v1beta1::PoolAsset, poolmanager::v1beta1::PoolResponse};
use pyth_sdk_cw::PriceIdentifier;

pub fn setup_test_with_pools() -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
    let mut deps = setup_test();

    // set a few osmosis pools
    let assets = vec![coin(42069, "uatom"), coin(69420, "uosmo")];
    deps.querier.set_query_pool_response(
        1,
        prepare_query_balancer_pool_response(
            1,
            &assets,
            &[5000u64, 5000u64],
            &coin(10000, "gamm/pool/1"),
        ),
    );

    let assets = vec![coin(12345, "uusdc"), coin(23456, "uatom")];
    deps.querier.set_query_pool_response(
        64,
        prepare_query_balancer_pool_response(
            64,
            &assets,
            &[5000u64, 5000u64],
            &coin(10000, "gamm/pool/64"),
        ),
    );

    let assets = vec![coin(12345, "uosmo"), coin(88888, "umars")];
    deps.querier.set_query_pool_response(
        89,
        prepare_query_balancer_pool_response(
            89,
            &assets,
            &[5000u64, 5000u64],
            &coin(10000, "gamm/pool/89"),
        ),
    );

    let assets = vec![coin(12345, "ustatom"), coin(88888, "uatom")];
    deps.querier.set_query_pool_response(
        803,
        prepare_query_balancer_pool_response(
            803,
            &assets,
            &[5000u64, 5000u64],
            &coin(10000, "gamm/pool/803"),
        ),
    );

    let assets = vec![coin(100000, "uusdc"), coin(100000, "uusdt"), coin(100000, "udai")];
    deps.querier.set_query_pool_response(
        3333,
        prepare_query_balancer_pool_response(
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
        prepare_query_balancer_pool_response(
            4444,
            &assets,
            &[5000u64, 5005u64],
            &coin(10000, "gamm/pool/4444"),
        ),
    );

    // Set StableSwap pool
    let assets = vec![coin(42069, "uatom"), coin(69420, "uosmo")];
    deps.querier
        .set_query_pool_response(5555, prepare_query_stable_swap_pool_response(5555, &assets));

    deps
}

pub fn setup_test_for_pyth() -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
    let mut deps = setup_test();

    // price source used to convert USD to base_denom
    set_price_source(
        deps.as_mut(),
        "usd",
        OsmosisPriceSourceUnchecked::Fixed {
            price: Decimal::from_str("1000000").unwrap(),
        },
    );

    deps
}

pub fn setup_test() -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
    let mut deps = OwnedDeps::<_, _, _> {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MarsMockQuerier::new(MockQuerier::new(&[])),
        custom_query_type: PhantomData,
    };

    // instantiate the oracle contract
    entry::instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        InstantiateMsg {
            owner: "owner".to_string(),
            base_denom: "uosmo".to_string(),
            custom_init: None,
        },
    )
    .unwrap();

    deps
}

pub fn prepare_query_balancer_pool_response(
    pool_id: u64,
    assets: &[Coin],
    weights: &[u64],
    shares: &Coin,
) -> PoolResponse {
    let pool = BalancerPool {
        address: "address".to_string(),
        id: pool_id,
        pool_params: None,
        future_pool_governor: "future_pool_governor".to_string(),
        total_shares: Some(osmosis_std::types::cosmos::base::v1beta1::Coin {
            denom: shares.denom.clone(),
            amount: shares.amount.to_string(),
        }),
        pool_assets: prepare_pool_assets(assets, weights),
        total_weight: "".to_string(),
    };
    PoolResponse {
        pool: Some(pool.to_any()),
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

pub fn prepare_query_stable_swap_pool_response(pool_id: u64, assets: &[Coin]) -> PoolResponse {
    let pool_liquidity: Vec<_> = assets
        .iter()
        .map(|coin| osmosis_std::types::cosmos::base::v1beta1::Coin {
            denom: coin.denom.clone(),
            amount: coin.amount.to_string(),
        })
        .collect();

    let pool = StableSwapPool {
        address: "osmo15v4mn84s9flhzpstkf9ql2mu0rnxh42pm8zhq47kh2fzs5zlwjsqaterkr".to_string(),
        id: pool_id,
        pool_params: None,
        future_pool_governor: "".to_string(),
        total_shares: Some(osmosis_std::types::cosmos::base::v1beta1::Coin {
            denom: format!("gamm/pool/{pool_id}"),
            amount: 4497913440357232330148u128.to_string(),
        }),
        pool_liquidity,
        scaling_factors: vec![100000u64, 113890u64],
        scaling_factor_controller: "osmo1k8c2m5cn322akk5wy8lpt87dd2f4yh9afcd7af".to_string(),
    };
    PoolResponse {
        pool: Some(pool.to_any()),
    }
}

pub fn set_pyth_price_source(deps: DepsMut, denom: &str, price_id: PriceIdentifier) {
    set_price_source(
        deps,
        denom,
        OsmosisPriceSourceUnchecked::Pyth {
            contract_addr: "pyth_contract".to_string(),
            price_feed_id: price_id,
            max_staleness: 30,
            max_confidence: Decimal::percent(10u64),
            max_deviation: Decimal::percent(15u64),
            denom_decimals: 6,
        },
    )
}

pub fn set_price_source(deps: DepsMut, denom: &str, price_source: OsmosisPriceSourceUnchecked) {
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
