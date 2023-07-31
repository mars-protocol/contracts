use std::str::FromStr;

use cosmwasm_std::{Addr, Decimal};
use mars_owner::OwnerError;
use mars_params::error::ContractError::{Owner, Validation};
use mars_utils::error::ValidationError::InvalidParam;

use crate::helpers::{assert_err, MockEnv};

pub mod helpers;

#[test]
fn thf_set_on_init() {
    let mock = MockEnv::new().build().unwrap();
    let thf = mock.query_target_health_factor();
    assert_eq!(thf, Decimal::from_str("1.05").unwrap())
}

#[test]
fn thf_validated_on_init() {
    let res = MockEnv::new().target_health_factor(Decimal::from_str("0.99").unwrap()).build();
    if res.is_ok() {
        panic!("Should have thrown an instantiate error");
    }
}

#[test]
fn only_owner_can_update_thf() {
    let mut mock = MockEnv::new().build().unwrap();
    let bad_guy = Addr::unchecked("doctor_otto_983");
    let res = mock.update_target_health_factor(&bad_guy, Decimal::from_str("1.1").unwrap());
    assert_err(res, Owner(OwnerError::NotOwner {}));
}

#[test]
fn validated_updates() {
    let mut mock = MockEnv::new().build().unwrap();

    let res =
        mock.update_target_health_factor(&mock.query_owner(), Decimal::from_str("0.99").unwrap());
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "target_health_factor".to_string(),
            invalid_value: "0.99".to_string(),
            predicate: "[1, 2]".to_string(),
        }),
    );

    let res =
        mock.update_target_health_factor(&mock.query_owner(), Decimal::from_str("2.01").unwrap());
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "target_health_factor".to_string(),
            invalid_value: "2.01".to_string(),
            predicate: "[1, 2]".to_string(),
        }),
    );
}

#[test]
fn update_thf() {
    let mut mock = MockEnv::new().build().unwrap();
    let target_health_factor = Decimal::from_str("1.08").unwrap();
    let current_thf = mock.query_target_health_factor();
    assert_ne!(current_thf, target_health_factor);

    mock.update_target_health_factor(&mock.query_owner(), Decimal::from_str("1.08").unwrap())
        .unwrap();

    let current_thf = mock.query_target_health_factor();
    assert_eq!(current_thf, target_health_factor);
}
