use cosmwasm_std::testing::mock_env;
use mars_oracle_base::ContractError;
use mars_oracle_osmosis::contract::entry;
use mars_red_bank_types::oracle::{ConfigResponse, InstantiateMsg, QueryMsg};
use mars_testing::{mock_dependencies, mock_info};

mod helpers;

#[test]
fn instantiating() {
    let deps = helpers::setup_test();

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
        },
    );
    assert_eq!(
        res,
        Err(ContractError::InvalidDenom {
            reason: "Invalid denom length".to_string()
        })
    );
}
