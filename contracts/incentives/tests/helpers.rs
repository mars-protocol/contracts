#![allow(dead_code)]

use cosmwasm_schema::serde;
use cosmwasm_std::{
    from_binary,
    testing::{mock_env, mock_info, MockApi, MockStorage},
    Deps, DepsMut, Env, OwnedDeps, Uint128,
};
use mars_incentives::contract::{execute, instantiate, query};
use mars_red_bank_types::incentives::{ExecuteMsg, InstantiateMsg, QueryMsg};
use mars_testing::{mock_dependencies, MarsMockQuerier};

pub fn th_setup() -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
    th_setup_with_env(mock_env())
}

pub fn th_setup_with_env(env: Env) -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: String::from("owner"),
        address_provider: String::from("address_provider"),
        epoch_duration: 604800, // 1 week in seconds
        min_incentive_emission: Uint128::from(100u128),
    };
    let info = mock_info("owner", &[]);
    instantiate(deps.as_mut(), env, info, msg).unwrap();

    deps
}

pub fn ths_setup_with_epoch_duration(
    env: Env,
    epoch_duration: u64,
) -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: String::from("owner"),
        address_provider: String::from("address_provider"),
        epoch_duration,
        min_incentive_emission: Uint128::from(100u128),
    };
    let info = mock_info("owner", &[]);
    instantiate(deps.as_mut(), env, info, msg).unwrap();

    deps
}

pub fn th_query<T: serde::de::DeserializeOwned>(deps: Deps, msg: QueryMsg) -> T {
    from_binary(&query(deps, mock_env(), msg).unwrap()).unwrap()
}

pub fn th_query_with_env<T: serde::de::DeserializeOwned>(deps: Deps, env: Env, msg: QueryMsg) -> T {
    from_binary(&query(deps, env, msg).unwrap()).unwrap()
}

pub fn th_whitelist_denom(deps: DepsMut, denom: &str) {
    let owner = "owner";
    let msg = ExecuteMsg::UpdateWhitelist {
        add_denoms: vec![denom.to_string()],
        remove_denoms: vec![],
    };
    let info = mock_info(owner, &[]);
    execute(deps, mock_env(), info, msg).unwrap();
}
