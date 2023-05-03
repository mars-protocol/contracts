use cosmwasm_std::coin;
use mars_owner::OwnerUpdate;
use osmosis_test_tube::Account;

use crate::helpers::{assert_err, MockEnv};

pub mod helpers;

#[test]
fn owner_set_on_init() {
    let mock = MockEnv::new().build().unwrap();
    assert!(mock.query_ownership().owner.is_some());
}

#[test]
fn only_owner_can_update_ownership() {
    let mut mock = MockEnv::new().build().unwrap();
    let bad_guy = mock.app.init_account(&[coin(1_000_000, "uosmo")]).unwrap();
    let err = mock
        .update_owner(
            OwnerUpdate::ProposeNewOwner {
                proposed: bad_guy.address(),
            },
            Some(&bad_guy),
        )
        .unwrap_err();

    assert_err(err, "Caller is not owner")
}

#[test]
fn owner_can_be_updated() {
    let mut mock = MockEnv::new().build().unwrap();

    let ownership = mock.query_ownership();
    assert_eq!(ownership.proposed, None);

    let new_owner = mock.app.init_account(&[coin(1_000_000_000, "uosmo")]).unwrap();
    mock.update_owner(
        OwnerUpdate::ProposeNewOwner {
            proposed: new_owner.address(),
        },
        None,
    )
    .unwrap();

    let ownership = mock.query_ownership();
    assert_eq!(ownership.proposed, Some(new_owner.address()));

    mock.update_owner(OwnerUpdate::AcceptProposed {}, Some(&new_owner)).unwrap();
    let ownership = mock.query_ownership();
    assert_eq!(ownership.owner, Some(new_owner.address()));
    assert_eq!(ownership.proposed, None);
}
