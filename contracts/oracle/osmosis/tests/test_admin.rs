use cosmwasm_std::{attr, testing::mock_env};
use mars_oracle::msg::{ConfigResponse, InstantiateMsg, QueryMsg};
use mars_oracle_base::ContractError;
use mars_oracle_osmosis::{contract::entry, msg::ExecuteMsg};
use mars_owner::OwnerError::NotOwner;
use mars_testing::{mock_dependencies, mock_info};

mod helpers;

#[test]
fn instantiating() {
    let deps = helpers::setup_test_with_pools();

    let cfg: ConfigResponse = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(cfg.owner.unwrap(), "owner".to_string());
    assert_eq!(cfg.proposed_new_owner, None);
    assert_eq!(cfg.base_denom, "uosmo".to_string());
}

#[test]
fn instantiating_incorrect_denom() {
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
            custom_init: None,
        },
    );
    assert_eq!(
        res,
        Err(ContractError::InvalidDenom {
            reason: "First character is not ASCII alphabetic".to_string()
        })
    );

    let res = entry::instantiate(
        deps.as_mut(),
        env.clone(),
        owner.clone(),
        InstantiateMsg {
            owner: "owner".to_string(),
            base_denom: "ahdbufenf&*!-".to_string(),
            custom_init: None,
        },
    );
    assert_eq!(
        res,
        Err(ContractError::InvalidDenom {
            reason: "Not all characters are ASCII alphanumeric or one of:  /  :  .  _  -"
                .to_string()
        })
    );

    let res = entry::instantiate(
        deps.as_mut(),
        env,
        owner,
        InstantiateMsg {
            owner: "owner".to_string(),
            base_denom: "ab".to_string(),
            custom_init: None,
        },
    );
    assert_eq!(
        res,
        Err(ContractError::InvalidDenom {
            reason: "Invalid denom length".to_string()
        })
    );
}

#[test]
fn update_config_if_unauthorized() {
    let mut deps = helpers::setup_test();

    let msg = ExecuteMsg::UpdateConfig {
        base_denom: None,
    };
    let info = mock_info("somebody");
    let res_err = entry::execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res_err, ContractError::Owner(NotOwner {}));
}

#[test]
fn update_config_with_invalid_base_denom() {
    let mut deps = helpers::setup_test();

    let msg = ExecuteMsg::UpdateConfig {
        base_denom: Some("*!fdskfna".to_string()),
    };
    let info = mock_info("owner");
    let res_err = entry::execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        res_err,
        ContractError::InvalidDenom {
            reason: "First character is not ASCII alphabetic".to_string()
        }
    );
}

#[test]
fn update_config_with_new_params() {
    let mut deps = helpers::setup_test();

    let msg = ExecuteMsg::UpdateConfig {
        base_denom: Some("uusdc".to_string()),
    };
    let info = mock_info("owner");
    let res = entry::execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(res.messages.len(), 0);
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "update_config"),
            attr("prev_base_denom", "uosmo"),
            attr("base_denom", "uusdc"),
        ]
    );

    let cfg: ConfigResponse = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(cfg.owner.unwrap(), "owner".to_string());
    assert_eq!(cfg.proposed_new_owner, None);
    assert_eq!(cfg.base_denom, "uusdc".to_string());
}
