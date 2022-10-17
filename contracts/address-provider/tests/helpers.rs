use cosmwasm_std::testing::{
    mock_dependencies_with_balance, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{from_binary, Addr, Deps, OwnedDeps};
use cw_multi_test::BasicApp;
use mars_address_provider::contract::{instantiate, query};

use mars_outpost::address_provider::{InstantiateMsg, QueryMsg};

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
    from_binary(&query(deps, mock_env(), msg).unwrap()).unwrap()
}

pub fn instantiate_address_provider(app: &mut BasicApp) -> Addr {
    // mars_testing::integration::mock_multitest::deploy_address_provider(app)
    unimplemented!()
}
