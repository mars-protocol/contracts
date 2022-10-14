use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage};
use cosmwasm_std::{Addr, OwnedDeps};
use cw_multi_test::BasicApp;
use mars_incentives::contract::instantiate;

use mars_outpost::incentives::InstantiateMsg;
use mars_testing::{mock_dependencies, MarsMockQuerier};

pub fn setup_test() -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: String::from("owner"),
        address_provider: String::from("address_provider"),
        mars_denom: String::from("umars"),
    };
    let info = mock_info("owner", &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps
}

pub fn instantiate_incentives(app: &mut BasicApp) -> Addr {
    mars_testing::multitest::mock_multitest::deploy_incentives(app)
}
