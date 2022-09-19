use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{BankMsg, Coin, CosmosMsg, Decimal, SubMsg, Uint128};

use mars_outpost::error::MarsError;
use mars_outpost::revenue_collector::{Config, CreateOrUpdateConfig, QueryMsg};
use mars_revenue_collector_base::ContractError;
use mars_revenue_collector_osmosis::contract::entry::{execute, instantiate};
use mars_revenue_collector_osmosis::msg::ExecuteMsg;
use mars_testing::mock_info;

use crate::helpers::mock_config;

mod helpers;

#[test]
fn test_instantiating() {
    let mut deps = helpers::setup_test();

    // config should have been correctly stored
    let cfg: Config<String> = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(cfg, mock_config().into());

    // init config with safety_tax_rate greater than 1; should fail
    let mut cfg = mock_config();
    cfg.safety_tax_rate = Decimal::percent(150);

    let info = mock_info("deployer");
    let err = instantiate(deps.as_mut(), mock_env(), info, cfg.into()).unwrap_err();
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
fn test_updating_config() {
    let mut deps = helpers::setup_test();

    let new_cfg = CreateOrUpdateConfig {
        safety_tax_rate: Some(Decimal::percent(69)),
        ..Default::default()
    };

    // non-owner is not authorized
    let info = mock_info("jake");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: new_cfg.clone(),
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, MarsError::Unauthorized {}.into());

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

    let cfg: Config<String> = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(cfg.safety_tax_rate, Decimal::percent(69));
}

#[test]
fn test_executing_cosmos_msg() {
    let mut deps = helpers::setup_test();

    let cosmos_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: "destination".to_string(),
        amount: vec![Coin {
            denom: "uluna".to_string(),
            amount: Uint128::new(123456),
        }],
    });
    let msg = ExecuteMsg::ExecuteCosmosMsg {
        cosmos_msg: cosmos_msg.clone(),
    };

    // non-owner is not authorized
    let err = execute(deps.as_mut(), mock_env(), mock_info("jake"), msg.clone()).unwrap_err();
    assert_eq!(err, MarsError::Unauthorized {}.into());

    // owner can execute cosmos msg
    let res = execute(deps.as_mut(), mock_env(), mock_info("owner"), msg).unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(res.messages[0], SubMsg::new(cosmos_msg));
}
