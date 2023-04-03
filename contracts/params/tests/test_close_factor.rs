use std::str::FromStr;

use cosmwasm_std::{Addr, Decimal};
use mars_owner::{OwnerError, OwnerUpdate};
use mars_params::error::ContractError::{Owner, Validation};
use mars_utils::error::ValidationError::InvalidParam;

use crate::helpers::{assert_err, MockEnv};

pub mod helpers;

#[test]
fn mcf_set_on_init() {
    let mock = MockEnv::new().build().unwrap();
    let mcf = mock.query_max_close_factor();
    assert_eq!(mcf, Decimal::from_str("0.5").unwrap())
}

#[test]
fn mcf_validated_on_init() {
    let res = MockEnv::new().max_close_factor(Decimal::from_str("1.23").unwrap()).build();
    if res.is_ok() {
        panic!("Should have thrown an instantiate error");
    }
}

#[test]
fn only_owner_can_update_mcf() {
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
fn validated_updates() {
    let mut mock = MockEnv::new().build().unwrap();
    let res = mock.update_max_close_factor(&mock.query_owner(), Decimal::from_str("1.9").unwrap());
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "max-close-factor".to_string(),
            invalid_value: "max-close-factor".to_string(),
            predicate: "<= 1".to_string(),
        }),
    );
}

#[test]
fn update_mcf() {
    let mut mock = MockEnv::new().build().unwrap();
    let new_max_close_factor = Decimal::from_str("0.9").unwrap();
    let current_mcf = mock.query_max_close_factor();
    assert_ne!(current_mcf, new_max_close_factor);

    mock.update_max_close_factor(&mock.query_owner(), Decimal::from_str("0.9").unwrap()).unwrap();

    let current_mcf = mock.query_max_close_factor();
    assert_eq!(current_mcf, new_max_close_factor);
}
