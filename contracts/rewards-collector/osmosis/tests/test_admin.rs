use cosmwasm_std::{testing::mock_env, Decimal};
use mars_outpost::{
    error::MarsError,
    rewards_collector::{ConfigResponse, QueryMsg, UpdateConfig},
};
use mars_owner::OwnerError::NotOwner;
use mars_rewards_collector_base::ContractError;
use mars_rewards_collector_osmosis::{
    contract::entry::{execute, instantiate},
    msg::ExecuteMsg,
};
use mars_testing::mock_info;

use crate::helpers::{mock_config, mock_instantiate_msg};

mod helpers;

#[test]
fn test_instantiating() {
    let mut deps = helpers::setup_test();

    let mut init_msg = mock_instantiate_msg();
    let config = mock_config(deps.api, init_msg.clone());

    // config should have been correctly stored
    let cfg: ConfigResponse = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(
        cfg,
        ConfigResponse {
            owner: Some("owner".to_string()),
            proposed_new_owner: None,
            address_provider: config.address_provider.to_string(),
            safety_tax_rate: config.safety_tax_rate,
            safety_fund_denom: config.safety_fund_denom,
            fee_collector_denom: config.fee_collector_denom,
            channel_id: config.channel_id,
            timeout_revision: config.timeout_revision,
            timeout_blocks: config.timeout_blocks,
            timeout_seconds: config.timeout_seconds,
            slippage_tolerance: config.slippage_tolerance,
        }
    );

    // init config with safety_tax_rate greater than 1; should fail
    init_msg.safety_tax_rate = Decimal::percent(150);

    let info = mock_info("deployer");
    let err = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::Mars(MarsError::InvalidParam {
            param_name: "safety_tax_rate".to_string(),
            invalid_value: "1.5".to_string(),
            predicate: "<= 1".to_string(),
        })
    );
}

#[test]
fn test_updating_config_if_invalid_slippage() {
    let mut deps = helpers::setup_test();

    let invalid_cfg = UpdateConfig {
        slippage_tolerance: Some(Decimal::percent(51u64)),
        ..Default::default()
    };

    let info = mock_info("owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: invalid_cfg,
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::Mars(MarsError::InvalidParam {
            param_name: "slippage_tolerance".to_string(),
            invalid_value: "0.51".to_string(),
            predicate: "<= 0.5".to_string(),
        })
    );
}

#[test]
fn test_updating_config() {
    let mut deps = helpers::setup_test();

    let new_cfg = UpdateConfig {
        safety_tax_rate: Some(Decimal::percent(69)),
        ..Default::default()
    };

    // non-owner is not authorized
    let info = mock_info("jake");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: new_cfg.clone(),
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, ContractError::Owner(NotOwner {}));

    // update config with safety_tax_rate greater than 1
    let mut invalid_cfg = new_cfg.clone();
    invalid_cfg.safety_tax_rate = Some(Decimal::percent(125));

    let info = mock_info("owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: invalid_cfg,
    };
    let err = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::Mars(MarsError::InvalidParam {
            param_name: "safety_tax_rate".to_string(),
            invalid_value: "1.25".to_string(),
            predicate: "<= 1".to_string(),
        })
    );

    // update config properly
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg,
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let cfg: ConfigResponse = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(cfg.safety_tax_rate, Decimal::percent(69));
}

#[test]
fn test_updating_config_if_invalid_timeout_revision() {
    let mut deps = helpers::setup_test();

    let invalid_cfg = UpdateConfig {
        timeout_revision: Some(0),
        ..Default::default()
    };

    let info = mock_info("owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: invalid_cfg,
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::Mars(MarsError::InvalidParam {
            param_name: "timeout_revision".to_string(),
            invalid_value: "0".to_string(),
            predicate: "> 0".to_string(),
        })
    );
}

#[test]
fn test_updating_config_if_invalid_timeout_blocks() {
    let mut deps = helpers::setup_test();

    let invalid_cfg = UpdateConfig {
        timeout_blocks: Some(0),
        ..Default::default()
    };

    let info = mock_info("owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: invalid_cfg,
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::Mars(MarsError::InvalidParam {
            param_name: "timeout_blocks".to_string(),
            invalid_value: "0".to_string(),
            predicate: "> 0".to_string(),
        })
    );
}

#[test]
fn test_updating_config_if_invalid_timeout_seconds() {
    let mut deps = helpers::setup_test();

    let invalid_cfg = UpdateConfig {
        timeout_seconds: Some(0),
        ..Default::default()
    };

    let info = mock_info("owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: invalid_cfg,
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::Mars(MarsError::InvalidParam {
            param_name: "timeout_seconds".to_string(),
            invalid_value: "0".to_string(),
            predicate: "> 0".to_string(),
        })
    );
}
