use cosmwasm_std::{Addr, StdError};
use mars_owner::OwnerError::NotOwner;
use mars_rover_health_types::{
    HealthError,
    HealthError::{Owner, Std},
};

use crate::helpers::MockEnv;

pub mod helpers;

#[test]
fn only_owner_can_update_config() {
    let mut mock = MockEnv::new().build().unwrap();

    let new_cm_addr = Addr::unchecked("xyz");
    let bad_guy = Addr::unchecked("bad_guy");
    let err: HealthError =
        mock.update_config(&bad_guy, &new_cm_addr).unwrap_err().downcast().unwrap();

    assert_eq!(err, Owner(NotOwner {}));
}

#[test]
fn raises_on_invalid_config() {
    let mut mock = MockEnv::new().build().unwrap();

    let new_cm_addr = Addr::unchecked("");
    let err: HealthError =
        mock.update_config(&mock.deployer.clone(), &new_cm_addr).unwrap_err().downcast().unwrap();

    assert_eq!(
        err,
        Std(StdError::generic_err(
            "Invalid input: human address too short for this mock implementation (must be >= 3)."
        ))
    );
}

#[test]
fn update_config_works() {
    let mut mock = MockEnv::new().build().unwrap();

    let new_cm_addr = Addr::unchecked("abc");
    mock.update_config(&mock.deployer.clone(), &new_cm_addr).unwrap();

    let new_config = mock.query_config();

    assert_eq!(new_config.credit_manager_addr, Some("abc".to_string()));
    assert_eq!(new_config.owner_response.owner, Some(mock.deployer.to_string()));
    assert_eq!(new_config.owner_response.proposed, None);
    assert!(new_config.owner_response.initialized);
    assert!(!new_config.owner_response.abolished);
}
