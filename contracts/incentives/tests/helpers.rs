#![allow(dead_code)]

use cosmwasm_schema::serde;
use cosmwasm_std::{
    from_binary,
    testing::{mock_env, mock_info, MockApi, MockStorage},
    Deps, Env, OwnedDeps,
};
use mars_incentives::contract::{instantiate, query};
use mars_outpost::incentives::{InstantiateMsg, QueryMsg};
use mars_testing::{mock_dependencies, MarsMockQuerier};

pub fn th_setup() -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
    th_setup_with_env(mock_env())
}

pub fn th_setup_with_env(env: Env) -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: String::from("owner"),
        address_provider: String::from("address_provider"),
        mars_denom: String::from("umars"),
    };
    let info = mock_info("owner", &[]);
    instantiate(deps.as_mut(), env, info, msg).unwrap();

    deps
}

pub fn th_query<T: serde::de::DeserializeOwned>(deps: Deps, msg: QueryMsg) -> T {
    from_binary(&query(deps, mock_env(), msg).unwrap()).unwrap()
}
