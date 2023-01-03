use crate::helpers::{th_query, th_setup};
use cosmwasm_std::testing::{mock_env, mock_info};
use cw_controllers_admin_fork::AdminError::NotAdmin;
use cw_controllers_admin_fork::AdminUpdate;
use mars_incentives::contract::execute;
use mars_incentives::ContractError;
use mars_outpost::incentives::{ConfigResponse, ExecuteMsg, QueryMsg};

mod helpers;

#[test]
fn test_initialized_state() {
    let deps = th_setup();

    let config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});
    assert!(config.owner.is_some());
    assert!(config.proposed_new_owner.is_none());
}

#[test]
fn test_update_owner() {
    let mut deps = th_setup();

    let original_config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});

    let new_owner = "new_admin";

    // only owner can propose new owners
    let bad_guy = "bad_guy";
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(bad_guy, &[]),
        ExecuteMsg::UpdateOwner(AdminUpdate::ProposeNewAdmin {
            proposed: bad_guy.to_string(),
        }),
    )
    .unwrap_err();
    assert_eq!(err, ContractError::AdminError(NotAdmin {}));

    // propose new owner
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(&original_config.owner.clone().unwrap(), &[]),
        ExecuteMsg::UpdateOwner(AdminUpdate::ProposeNewAdmin {
            proposed: new_owner.to_string(),
        }),
    )
    .unwrap();
    let new_config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(new_config.owner.unwrap(), original_config.owner.clone().unwrap());
    assert_ne!(new_config.proposed_new_owner, original_config.proposed_new_owner);
    assert_eq!(new_config.proposed_new_owner.unwrap(), new_owner.to_string());

    // accept ownership
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(new_owner, &[]),
        ExecuteMsg::UpdateOwner(AdminUpdate::AcceptProposed),
    )
    .unwrap();
    let new_config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(new_config.owner.clone().unwrap(), new_owner.to_string());
    assert_ne!(new_config.owner, original_config.owner);
    assert_eq!(new_config.proposed_new_owner, None);
}
