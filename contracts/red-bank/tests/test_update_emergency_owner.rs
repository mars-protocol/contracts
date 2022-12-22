use crate::helpers::{th_query, th_setup};
use cosmwasm_std::testing::{mock_env, mock_info};
use cw_controllers_admin_fork::AdminError::{NotAdmin, NotProposedAdmin};
use cw_controllers_admin_fork::AdminUpdate;
use mars_outpost::red_bank::{ConfigResponse, ExecuteMsg, QueryMsg};
use mars_red_bank::contract::execute;
use mars_red_bank::error::ContractError;

mod helpers;

#[test]
fn test_initialized_state() {
    let deps = th_setup(&[]);

    let config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});
    assert!(config.emergency_owner.is_some());
    assert!(config.proposed_new_emergency_owner.is_none());
}

#[test]
fn test_propose_new_emergency_owner() {
    let mut deps = th_setup(&[]);

    let original_config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});

    let new_admin = "new_admin";

    // only admin can propose new admins
    let bad_guy = "bad_guy";
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(bad_guy, &[]),
        ExecuteMsg::UpdateEmergencyOwner(AdminUpdate::ProposeNewAdmin {
            proposed: bad_guy.to_string(),
        }),
    )
    .unwrap_err();
    assert_eq!(err, ContractError::AdminError(NotAdmin {}));

    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(&original_config.emergency_owner.clone().unwrap(), &[]),
        ExecuteMsg::UpdateEmergencyOwner(AdminUpdate::ProposeNewAdmin {
            proposed: new_admin.to_string(),
        }),
    )
    .unwrap();

    let new_config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});

    assert_eq!(new_config.owner, original_config.owner);
    assert_eq!(new_config.proposed_new_owner, original_config.proposed_new_owner);
    assert_eq!(new_config.emergency_owner, original_config.emergency_owner);
    assert_ne!(
        new_config.proposed_new_emergency_owner,
        original_config.proposed_new_emergency_owner
    );
    assert_eq!(new_config.proposed_new_emergency_owner, Some(new_admin.to_string()));
}

#[test]
fn test_clear_proposed_emergency_owner() {
    let mut deps = th_setup(&[]);

    let original_config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});

    let new_admin = "new_admin";

    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(&original_config.emergency_owner.clone().unwrap(), &[]),
        ExecuteMsg::UpdateEmergencyOwner(AdminUpdate::ProposeNewAdmin {
            proposed: new_admin.to_string(),
        }),
    )
    .unwrap();

    let interim_config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(interim_config.proposed_new_emergency_owner, Some(new_admin.to_string()));

    // only admin can clear
    let bad_guy = "bad_guy";
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(bad_guy, &[]),
        ExecuteMsg::UpdateEmergencyOwner(AdminUpdate::ClearProposed),
    )
    .unwrap_err();
    assert_eq!(err, ContractError::AdminError(NotAdmin {}));

    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(&original_config.emergency_owner.clone().unwrap(), &[]),
        ExecuteMsg::UpdateEmergencyOwner(AdminUpdate::ClearProposed),
    )
    .unwrap();

    let latest_config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});

    assert_eq!(latest_config.owner, original_config.owner);
    assert_eq!(latest_config.proposed_new_owner, original_config.proposed_new_owner);
    assert_eq!(latest_config.emergency_owner, original_config.emergency_owner);
    assert_ne!(
        latest_config.proposed_new_emergency_owner,
        interim_config.proposed_new_emergency_owner
    );
    assert_eq!(latest_config.proposed_new_emergency_owner, None);
}

#[test]
fn test_accept_emergency_owner_role() {
    let mut deps = th_setup(&[]);

    let original_config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});

    let new_admin = "new_admin";

    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(&original_config.emergency_owner.clone().unwrap(), &[]),
        ExecuteMsg::UpdateEmergencyOwner(AdminUpdate::ProposeNewAdmin {
            proposed: new_admin.to_string(),
        }),
    )
    .unwrap();

    // Only proposed admin can accept
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(&original_config.emergency_owner.unwrap(), &[]),
        ExecuteMsg::UpdateEmergencyOwner(AdminUpdate::AcceptProposed),
    )
    .unwrap_err();
    assert_eq!(err, ContractError::AdminError(NotProposedAdmin {}));

    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(new_admin, &[]),
        ExecuteMsg::UpdateEmergencyOwner(AdminUpdate::AcceptProposed),
    )
    .unwrap();

    let new_config: ConfigResponse = th_query(deps.as_ref(), QueryMsg::Config {});

    assert_eq!(new_config.owner, original_config.owner);
    assert_eq!(new_config.proposed_new_owner, original_config.proposed_new_owner);
    assert_eq!(new_config.emergency_owner.unwrap(), new_admin.to_string());
    assert_eq!(new_config.proposed_new_emergency_owner, None);
}
