use std::str::FromStr;

use cosmwasm_std::{Decimal, StdError::GenericErr};
use mars_params::{
    error::ContractError::{Std, Validation},
    types::VaultConfigUpdate,
};
use mars_utils::error::ValidationError::InvalidParam;

use crate::helpers::{assert_err, default_vault_config, MockEnv};

pub mod helpers;

#[test]
fn vault_addr_must_be_valid() {
    let mut mock = MockEnv::new().build().unwrap();

    let res = mock.update_vault_config(
        &mock.query_owner(),
        VaultConfigUpdate::AddOrUpdate {
            config: default_vault_config("%"),
        },
    );
    assert_err(
        res,
        Std(GenericErr { msg: "Invalid input: human address too short for this mock implementation (must be >= 3).".to_string() }),
    );
}

#[test]
fn vault_max_ltv_less_than_or_equal_to_one() {
    let mut mock = MockEnv::new().build().unwrap();
    let mut config = default_vault_config("vault_xyz");
    config.max_loan_to_value = Decimal::from_str("1.1235").unwrap();

    let res = mock.update_vault_config(
        &mock.query_owner(),
        VaultConfigUpdate::AddOrUpdate {
            config,
        },
    );
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "max_loan_to_value".to_string(),
            invalid_value: "1.1235".to_string(),
            predicate: "<= 1".to_string(),
        }),
    );
}

#[test]
fn vault_liquidation_threshold_less_than_or_equal_to_one() {
    let mut mock = MockEnv::new().build().unwrap();
    let mut config = default_vault_config("vault_xyz");
    config.liquidation_threshold = Decimal::from_str("1.1235").unwrap();

    let res = mock.update_vault_config(
        &mock.query_owner(),
        VaultConfigUpdate::AddOrUpdate {
            config,
        },
    );
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "liquidation_threshold".to_string(),
            invalid_value: "1.1235".to_string(),
            predicate: "<= 1".to_string(),
        }),
    );
}

#[test]
fn vault_liq_threshold_gt_max_ltv() {
    let mut mock = MockEnv::new().build().unwrap();
    let mut config = default_vault_config("vault_xyz");
    config.liquidation_threshold = Decimal::from_str("0.5").unwrap();
    config.max_loan_to_value = Decimal::from_str("0.6").unwrap();

    let res = mock.update_vault_config(
        &mock.query_owner(),
        VaultConfigUpdate::AddOrUpdate {
            config,
        },
    );
    assert_err(
        res,
        Validation(InvalidParam {
            param_name: "liquidation_threshold".to_string(),
            invalid_value: "0.5".to_string(),
            predicate: "> 0.6 (max LTV)".to_string(),
        }),
    );
}
