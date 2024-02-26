#![allow(dead_code)]

use cosmwasm_std::{
    from_json,
    testing::{
        mock_dependencies_with_balance, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    },
    Deps, OwnedDeps,
};
use mars_address_provider::contract::{instantiate, query};
use mars_types::address_provider::{InstantiateMsg, QueryMsg};

pub fn th_setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies_with_balance(&[]);

    instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("deployer", &[]),
        InstantiateMsg {
            owner: "osmo_owner".to_string(),
            prefix: "osmo".to_string(),
        },
    )
    .unwrap();

    deps
}

pub fn th_query<T: serde::de::DeserializeOwned>(deps: Deps, msg: QueryMsg) -> T {
    from_json(query(deps, mock_env(), msg).unwrap()).unwrap()
}
