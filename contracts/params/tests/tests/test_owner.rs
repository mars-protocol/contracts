use cosmwasm_std::Addr;
use mars_owner::{OwnerError, OwnerUpdate};
use mars_params::error::ContractError::Owner;

use super::helpers::{assert_err, MockEnv};

#[test]
fn owner_set_on_init() {
    let mock = MockEnv::new().build().unwrap();
    let owner = mock.query_owner();
    assert_eq!("owner", &owner.to_string())
}

#[test]
fn only_owner_can_execute_updates() {
    let mut mock = MockEnv::new().build().unwrap();
    let bad_guy = Addr::unchecked("doctor_otto_983");
    let res = mock.update_owner(
        &bad_guy,
        OwnerUpdate::ProposeNewOwner {
            proposed: bad_guy.to_string(),
        },
    );
    assert_err(res, Owner(OwnerError::NotOwner {}));
}

#[test]
fn owner_can_execute_updates() {
    let mut mock = MockEnv::new().build().unwrap();

    let ownership = mock.query_ownership();
    assert_eq!(ownership.emergency_owner, None);

    let em_owner = "miles_morales".to_string();
    mock.update_owner(
        &mock.query_owner(),
        OwnerUpdate::SetEmergencyOwner {
            emergency_owner: em_owner.clone(),
        },
    )
    .unwrap();

    let ownership = mock.query_ownership();
    assert_eq!(ownership.emergency_owner, Some(em_owner));
}
