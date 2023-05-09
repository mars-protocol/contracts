use cosmwasm_std::{Addr, Uint128};
use mars_account_nft::nft_config::NftConfigUpdates;

use crate::helpers::MockEnv;

pub mod helpers;

#[test]
fn only_minter_can_update_config() {
    let mut mock = MockEnv::new().build().unwrap();

    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.update_config(
        &bad_guy,
        &NftConfigUpdates {
            max_value_for_burn: None,
            health_contract_addr: None,
        },
    );

    if res.is_ok() {
        panic!("Non-minter should not be able to propose new minter");
    }
}

#[test]
fn minter_can_update_config() {
    let mut mock = MockEnv::new().build().unwrap();

    let new_max_burn_val = Uint128::new(4918453);
    let new_health_contract = "new_health_contract_123".to_string();

    let updates = NftConfigUpdates {
        max_value_for_burn: Some(new_max_burn_val),
        health_contract_addr: Some(new_health_contract.clone()),
    };

    mock.update_config(&mock.minter.clone(), &updates).unwrap();

    let config = mock.query_config();
    assert_eq!(config.max_value_for_burn, new_max_burn_val);
    assert_eq!(config.health_contract_addr.unwrap(), new_health_contract);
}
