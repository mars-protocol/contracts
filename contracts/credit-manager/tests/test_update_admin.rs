use cosmwasm_std::Addr;
use cw_controllers_admin_fork::AdminError::{NotAdmin, NotProposedAdmin, StateTransitionError};
use cw_controllers_admin_fork::AdminExecuteUpdate;
use mars_rover::error::ContractError::AdminError;

use crate::helpers::{assert_err, MockEnv};

pub mod helpers;

#[test]
fn test_initialized_state() {
    let mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    assert!(original_config.admin.is_some());
    assert!(original_config.proposed_new_admin.is_none());
}

#[test]
fn test_propose_new_admin() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    let new_admin = "new_admin".to_string();

    // only admin can propose new admins
    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.update_admin(
        &bad_guy,
        AdminExecuteUpdate::ProposeNewAdmin {
            proposed: bad_guy.to_string(),
        },
    );
    assert_err(res, AdminError(NotAdmin {}));

    mock.update_admin(
        &Addr::unchecked(original_config.admin.clone().unwrap()),
        AdminExecuteUpdate::ProposeNewAdmin {
            proposed: new_admin.clone(),
        },
    )
    .unwrap();

    let new_config = mock.query_config();

    assert_eq!(new_config.admin, original_config.admin);
    assert_ne!(
        new_config.proposed_new_admin,
        original_config.proposed_new_admin
    );
    assert_eq!(new_config.proposed_new_admin, Some(new_admin));
}

#[test]
fn test_clear_proposed() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    let new_admin = "new_admin".to_string();

    mock.update_admin(
        &Addr::unchecked(original_config.admin.clone().unwrap()),
        AdminExecuteUpdate::ProposeNewAdmin {
            proposed: new_admin.clone(),
        },
    )
    .unwrap();

    let interim_config = mock.query_config();

    assert_eq!(interim_config.proposed_new_admin, Some(new_admin));

    // only admin can clear
    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.update_admin(&bad_guy, AdminExecuteUpdate::ClearProposed);
    assert_err(res, AdminError(NotAdmin {}));

    mock.update_admin(
        &Addr::unchecked(original_config.admin.clone().unwrap()),
        AdminExecuteUpdate::ClearProposed,
    )
    .unwrap();

    let latest_config = mock.query_config();

    assert_eq!(latest_config.admin, original_config.admin);
    assert_ne!(
        latest_config.proposed_new_admin,
        interim_config.proposed_new_admin
    );
    assert_eq!(latest_config.proposed_new_admin, None);
}

#[test]
fn test_accept_admin_role() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    let new_admin = "new_admin".to_string();

    mock.update_admin(
        &Addr::unchecked(original_config.admin.clone().unwrap()),
        AdminExecuteUpdate::ProposeNewAdmin {
            proposed: new_admin.clone(),
        },
    )
    .unwrap();

    // Only proposed admin can accept
    let res = mock.update_admin(
        &Addr::unchecked(original_config.admin.unwrap()),
        AdminExecuteUpdate::AcceptProposed,
    );
    assert_err(res, AdminError(NotProposedAdmin {}));

    mock.update_admin(
        &Addr::unchecked(new_admin.clone()),
        AdminExecuteUpdate::AcceptProposed,
    )
    .unwrap();

    let new_config = mock.query_config();

    assert_eq!(new_config.admin.unwrap(), new_admin);
    assert_eq!(new_config.proposed_new_admin, None);
}

#[test]
fn test_abolish_admin_role() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    // Only admin can abolish role
    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.update_admin(&bad_guy, AdminExecuteUpdate::AbolishAdminRole);
    assert_err(res, AdminError(NotAdmin {}));

    mock.update_admin(
        &Addr::unchecked(original_config.admin.clone().unwrap()),
        AdminExecuteUpdate::AbolishAdminRole,
    )
    .unwrap();

    let new_config = mock.query_config();

    assert_eq!(new_config.admin, None);
    assert_eq!(new_config.proposed_new_admin, None);

    // No new updates can occur
    let res = mock.update_admin(
        &Addr::unchecked(original_config.admin.clone().unwrap()),
        AdminExecuteUpdate::InitializeAdmin {
            admin: original_config.admin.unwrap(),
        },
    );
    assert_err(res, AdminError(StateTransitionError {}));
}
