use cosmwasm_std::{Addr, Decimal};

use mars_account_nft::config::ConfigUpdates;

use crate::helpers::MockEnv;

pub mod helpers;

#[test]
fn test_only_minter_can_update_config() {
    let mut mock = MockEnv::new().build().unwrap();

    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.update_config(
        &bad_guy,
        &ConfigUpdates {
            max_value_for_burn: None,
            proposed_new_minter: None,
        },
    );

    if res.is_ok() {
        panic!("Non-owner should not be able to propose ownership transfer");
    }
}

#[test]
fn test_minter_can_update_config() {
    let mut mock = MockEnv::new().build().unwrap();

    let new_max_burn_val = Decimal::from_atomics(4918453u128, 2).unwrap();
    let new_proposed_minter = "new_proposed_minter".to_string();

    let updates = ConfigUpdates {
        max_value_for_burn: Some(new_max_burn_val),
        proposed_new_minter: Some(new_proposed_minter.clone()),
    };

    mock.update_config(&mock.minter.clone(), &updates).unwrap();

    let config = mock.query_config();
    assert_eq!(config.max_value_for_burn, new_max_burn_val);
    assert_eq!(config.proposed_new_minter.unwrap(), new_proposed_minter);
}
