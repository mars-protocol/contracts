use cosmwasm_std::testing::mock_env;
use mars_oracle_base::ContractError;
use mars_oracle_osmosis::contract::entry;
use mars_outpost::error::MarsError;
use mars_outpost::oracle::{Config, InstantiateMsg, QueryMsg};
use mars_testing::{mock_dependencies, mock_info};

use mars_oracle_osmosis::contract::entry::execute;
use mars_oracle_osmosis::msg::ExecuteMsg;

mod helpers;

#[test]
fn test_instantiating() {
    let deps = helpers::setup_test();

    let cfg: Config<String> = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(cfg.owner, "owner".to_string());
    assert_eq!(cfg.base_denom, "uosmo".to_string());
}

#[test]
fn test_instantiating_incorrect_denom() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let owner = mock_info("owner");

    let res = entry::instantiate(
        deps.as_mut(),
        env.clone(),
        owner.clone(),
        InstantiateMsg {
            owner: "owner".to_string(),
            base_denom: "!*jadfaefc".to_string(),
        },
    );
    assert_eq!(
        res,
        Err(ContractError::Mars(MarsError::InvalidDenom {
            reason: "First character is not ASCII alphabetic".to_string()
        }))
    );

    let res = entry::instantiate(
        deps.as_mut(),
        env.clone(),
        owner.clone(),
        InstantiateMsg {
            owner: "owner".to_string(),
            base_denom: "ahdbufenf&*!-".to_string(),
        },
    );
    assert_eq!(
        res,
        Err(ContractError::Mars(MarsError::InvalidDenom {
            reason: "Not all characters are ASCII alphanumeric or one of:  /  :  .  _  -"
                .to_string()
        }))
    );

    let res = entry::instantiate(
        deps.as_mut(),
        env,
        owner,
        InstantiateMsg {
            owner: "owner".to_string(),
            base_denom: "ab".to_string(),
        },
    );
    assert_eq!(
        res,
        Err(ContractError::Mars(MarsError::InvalidDenom {
            reason: "Invalid denom length".to_string()
        }))
    );
}

#[test]
fn test_updating_config() {
    let mut deps = helpers::setup_test();

    let msg = ExecuteMsg::UpdateConfig {
        owner: "new_owner".to_string(),
    };

    // non-owner cannot update
    let err = execute(deps.as_mut(), mock_env(), mock_info("jake"), msg.clone()).unwrap_err();
    assert_eq!(err, MarsError::Unauthorized {}.into());

    // owner can update
    let res = execute(deps.as_mut(), mock_env(), mock_info("owner"), msg).unwrap();
    assert_eq!(res.messages.len(), 0);

    let cfg: Config<String> = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(cfg.owner, "new_owner".to_string());
}
