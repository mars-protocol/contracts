use cosmwasm_std::Addr;
use mars_owner::{
    OwnerError::{NotOwner, NotProposedOwner, StateTransitionError},
    OwnerUpdate,
};
use mars_rover::error::ContractError::Owner;

use crate::helpers::{assert_err, MockEnv};

pub mod helpers;

#[test]
fn initialized_state() {
    let mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    assert!(original_config.ownership.owner.is_some());
    assert!(original_config.ownership.proposed.is_none());
}

#[test]
fn propose_new_owner() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    let new_owner = "new_owner".to_string();

    // only owner can propose new owners
    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.update_owner(
        &bad_guy,
        OwnerUpdate::ProposeNewOwner {
            proposed: bad_guy.to_string(),
        },
    );
    assert_err(res, Owner(NotOwner {}));

    mock.update_owner(
        &Addr::unchecked(original_config.ownership.owner.clone().unwrap()),
        OwnerUpdate::ProposeNewOwner {
            proposed: new_owner.clone(),
        },
    )
    .unwrap();

    let new_config = mock.query_config();

    assert_eq!(new_config.ownership.owner, original_config.ownership.owner);
    assert_ne!(new_config.ownership.proposed, original_config.ownership.proposed);
    assert_eq!(new_config.ownership.proposed, Some(new_owner));
}

#[test]
fn clear_proposed() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    let new_owner = "new_owner".to_string();

    mock.update_owner(
        &Addr::unchecked(original_config.ownership.owner.clone().unwrap()),
        OwnerUpdate::ProposeNewOwner {
            proposed: new_owner.clone(),
        },
    )
    .unwrap();

    let interim_config = mock.query_config();

    assert_eq!(interim_config.ownership.proposed, Some(new_owner));

    // only owner can clear
    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.update_owner(&bad_guy, OwnerUpdate::ClearProposed);
    assert_err(res, Owner(NotOwner {}));

    mock.update_owner(
        &Addr::unchecked(original_config.ownership.owner.clone().unwrap()),
        OwnerUpdate::ClearProposed,
    )
    .unwrap();

    let latest_config = mock.query_config();

    assert_eq!(latest_config.ownership.owner, original_config.ownership.owner);
    assert_ne!(latest_config.ownership.proposed, interim_config.ownership.proposed);
    assert_eq!(latest_config.ownership.proposed, None);
}

#[test]
fn accept_owner_role() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    let new_owner = "new_owner".to_string();

    mock.update_owner(
        &Addr::unchecked(original_config.ownership.owner.clone().unwrap()),
        OwnerUpdate::ProposeNewOwner {
            proposed: new_owner.clone(),
        },
    )
    .unwrap();

    // Only proposed owner can accept
    let res = mock.update_owner(
        &Addr::unchecked(original_config.ownership.owner.unwrap()),
        OwnerUpdate::AcceptProposed,
    );
    assert_err(res, Owner(NotProposedOwner {}));

    mock.update_owner(&Addr::unchecked(new_owner.clone()), OwnerUpdate::AcceptProposed).unwrap();

    let new_config = mock.query_config();

    assert_eq!(new_config.ownership.owner.unwrap(), new_owner);
    assert_eq!(new_config.ownership.proposed, None);
}

#[test]
fn abolish_owner_role() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    // Only owner can abolish role
    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.update_owner(&bad_guy, OwnerUpdate::AbolishOwnerRole);
    assert_err(res, Owner(NotOwner {}));

    mock.update_owner(
        &Addr::unchecked(original_config.ownership.owner.clone().unwrap()),
        OwnerUpdate::AbolishOwnerRole,
    )
    .unwrap();

    let new_config = mock.query_config();

    assert_eq!(new_config.ownership.owner, None);
    assert_eq!(new_config.ownership.proposed, None);

    // No new updates can occur
    let res = mock.update_owner(
        &Addr::unchecked(original_config.ownership.owner.clone().unwrap()),
        OwnerUpdate::ProposeNewOwner {
            proposed: original_config.ownership.owner.unwrap(),
        },
    );
    assert_err(res, Owner(StateTransitionError {}));
}
