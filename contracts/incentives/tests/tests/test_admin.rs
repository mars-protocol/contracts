use cosmwasm_std::{
    testing::{mock_env, mock_info},
    Addr, SubMsg,
};
use mars_incentives::{
    contract::{execute, instantiate},
    ContractError,
};
use mars_owner::OwnerError::NotOwner;
use mars_testing::mock_dependencies;
use mars_types::incentives::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};

use super::helpers::{th_query, th_setup};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let info = mock_info("sender", &[]);
    let msg = InstantiateMsg {
        owner: String::from("owner"),
        address_provider: String::from("address_provider"),
        epoch_duration: 604800, // 1 week in seconds
        max_whitelisted_denoms: 10,
    };

    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    let empty_vec: Vec<SubMsg> = vec![];
    assert_eq!(empty_vec, res.messages);

    let config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(config.owner, Some("owner".to_string()));
    assert_eq!(config.proposed_new_owner, None);
    assert_eq!(config.address_provider, "address_provider".to_string());
    assert_eq!(config.epoch_duration, 604800);
    assert_eq!(config.max_whitelisted_denoms, 10);
    assert_eq!(config.whitelist_count, 0);
}

#[test]
fn cant_instantiate_with_too_short_epoch_duration() {
    let mut deps = mock_dependencies(&[]);

    let info = mock_info("sender", &[]);
    let msg = InstantiateMsg {
        owner: String::from("owner"),
        address_provider: String::from("address_provider"),
        epoch_duration: 604800 - 1,
        max_whitelisted_denoms: 10,
    };

    let res = instantiate(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(
        res.unwrap_err(),
        ContractError::EpochDurationTooShort {
            min_epoch_duration: 604800
        }
    );
}

#[test]
fn update_config() {
    let mut deps = th_setup();

    // *
    // non owner is not authorized
    // *
    let msg = ExecuteMsg::UpdateConfig {
        address_provider: None,
        max_whitelisted_denoms: None,
    };
    let info = mock_info("somebody", &[]);
    let error_res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(error_res, ContractError::Owner(NotOwner {}));

    // *
    // update config with new params
    // *
    let msg = ExecuteMsg::UpdateConfig {
        address_provider: Some("new_addr_provider".to_string()),
        max_whitelisted_denoms: Some(20),
    };
    let info = mock_info("owner", &[]);

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // Read config from state
    let new_config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(new_config.owner, Some("owner".to_string()));
    assert_eq!(new_config.proposed_new_owner, None);
    assert_eq!(new_config.address_provider, Addr::unchecked("new_addr_provider"));
    assert_eq!(new_config.epoch_duration, 604800);
    assert_eq!(new_config.whitelist_count, 0);
    assert_eq!(new_config.max_whitelisted_denoms, 20);
}
