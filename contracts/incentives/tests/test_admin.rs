use cosmwasm_std::{
    testing::{mock_env, mock_info},
    Addr, SubMsg, Uint128,
};
use mars_incentives::{
    contract::{execute, instantiate},
    ContractError,
};
use mars_owner::OwnerError::NotOwner;
use mars_red_bank_types::incentives::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use mars_testing::mock_dependencies;
use mars_utils::error::ValidationError;

use crate::helpers::{th_query, th_setup};

mod helpers;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let info = mock_info("sender", &[]);
    let msg = InstantiateMsg {
        owner: String::from("owner"),
        address_provider: String::from("address_provider"),
        mars_denom: String::from("umars"),
        epoch_duration: 604800, // 1 week in seconds
        min_incentive_emission: Uint128::from(100u128),
    };

    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    let empty_vec: Vec<SubMsg> = vec![];
    assert_eq!(empty_vec, res.messages);

    let config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(config.owner, Some("owner".to_string()));
    assert_eq!(config.proposed_new_owner, None);
    assert_eq!(config.address_provider, "address_provider".to_string());
    assert_eq!(config.mars_denom, "umars".to_string());
}

#[test]
fn update_config() {
    let mut deps = th_setup();

    // *
    // non owner is not authorized
    // *
    let msg = ExecuteMsg::UpdateConfig {
        address_provider: None,
        mars_denom: None,
    };
    let info = mock_info("somebody", &[]);
    let error_res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(error_res, ContractError::Owner(NotOwner {}));

    // *
    // update config with invalid denom
    // *
    let msg = ExecuteMsg::UpdateConfig {
        address_provider: None,
        mars_denom: Some("*!fdskfna".to_string()),
    };
    let info = mock_info("owner", &[]);

    let err = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(
        err,
        Err(ContractError::Validation(ValidationError::InvalidDenom {
            reason: "First character is not ASCII alphabetic".to_string()
        }))
    );

    // *
    // update config with new params
    // *
    let msg = ExecuteMsg::UpdateConfig {
        address_provider: Some("new_addr_provider".to_string()),
        mars_denom: None,
    };
    let info = mock_info("owner", &[]);

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // Read config from state
    let new_config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(new_config.owner, Some("owner".to_string()));
    assert_eq!(new_config.proposed_new_owner, None);
    assert_eq!(new_config.address_provider, Addr::unchecked("new_addr_provider"));
    assert_eq!(new_config.mars_denom, "umars".to_string());
}
