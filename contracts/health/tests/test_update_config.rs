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

    let new_cm_addr = "xyz".to_string();
    let bad_guy = Addr::unchecked("bad_guy");
    let err: HealthError =
        mock.update_config(&bad_guy, Some(new_cm_addr), None).unwrap_err().downcast().unwrap();

    assert_eq!(err, Owner(NotOwner {}));
}

#[test]
fn raises_on_invalid_config() {
    let mut mock = MockEnv::new().build().unwrap();

    let err: HealthError = mock
        .update_config(&mock.deployer.clone(), Some("".to_string()), None)
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(
        err,
        Std(StdError::generic_err(
            "Invalid input: human address too short for this mock implementation (must be >= 3)."
        ))
    );
}

#[test]
fn update_partial_config_works() {
    let mut mock = MockEnv::new().skip_params_config().skip_cm_config().build().unwrap();

    mock.update_config(&mock.deployer.clone(), Some("abc".to_string()), None).unwrap();

    let new_config = mock.query_config();

    assert_eq!(new_config.credit_manager, Some("abc".to_string()));
    assert_eq!(new_config.params, None);
    assert_eq!(new_config.owner_response.owner, Some(mock.deployer.to_string()));
    assert_eq!(new_config.owner_response.proposed, None);
    assert!(new_config.owner_response.initialized);
    assert!(!new_config.owner_response.abolished);
}

#[test]
fn update_full_config_works() {
    let mut mock = MockEnv::new().skip_params_config().skip_cm_config().build().unwrap();

    mock.update_config(&mock.deployer.clone(), Some("abc".to_string()), Some("xyz".to_string()))
        .unwrap();

    let new_config = mock.query_config();

    assert_eq!(new_config.credit_manager, Some("abc".to_string()));
    assert_eq!(new_config.params, Some("xyz".to_string()));
    assert_eq!(new_config.owner_response.owner, Some(mock.deployer.to_string()));
    assert_eq!(new_config.owner_response.proposed, None);
    assert!(new_config.owner_response.initialized);
    assert!(!new_config.owner_response.abolished);
}
