use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{Addr, SubMsg};

use mars_outpost::error::MarsError;
use mars_outpost::incentives::{ExecuteMsg, InstantiateMsg};
use mars_testing::mock_dependencies;

use mars_incentives::contract::{execute, instantiate};
use mars_incentives::state::CONFIG;

use crate::helpers::setup_test;
use mars_incentives::ContractError;

mod helpers;

#[test]
fn test_proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let info = mock_info("sender", &[]);
    let msg = InstantiateMsg {
        owner: String::from("owner"),
        address_provider: String::from("address_provider"),
        mars_denom: String::from("umars"),
    };

    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    let empty_vec: Vec<SubMsg> = vec![];
    assert_eq!(empty_vec, res.messages);

    let config = CONFIG.load(deps.as_ref().storage).unwrap();
    assert_eq!(config.owner, Addr::unchecked("owner"));
    assert_eq!(config.mars_denom, "umars".to_string());
}

#[test]
fn test_update_config() {
    let mut deps = setup_test();

    // *
    // non owner is not authorized
    // *
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        address_provider: None,
        mars_denom: None,
    };
    let info = mock_info("somebody", &[]);
    let error_res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(error_res, ContractError::Mars(MarsError::Unauthorized {}));

    // *
    // update config with invalid denom
    // *
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some(String::from("new_owner")),
        address_provider: None,
        mars_denom: Some("*!fdskfna".to_string()),
    };
    let info = mock_info("owner", &[]);

    let err = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(
        err,
        Err(ContractError::Mars(MarsError::InvalidDenom {
            reason: "First character is not ASCII alphabetic".to_string()
        }))
    );

    // *
    // update config with new params
    // *
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some(String::from("new_owner")),
        address_provider: None,
        mars_denom: None,
    };
    let info = mock_info("owner", &[]);

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // Read config from state
    let new_config = CONFIG.load(deps.as_ref().storage).unwrap();
    assert_eq!(new_config.owner, Addr::unchecked("new_owner"));
    assert_eq!(new_config.mars_denom, "umars".to_string());
}
