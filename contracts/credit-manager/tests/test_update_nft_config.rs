extern crate core;

use cosmwasm_std::{Addr, Uint128};
use cw_multi_test::Executor;
use mars_owner::OwnerError::NotOwner;
use mars_rover::{
    adapters::account_nft::{ExecuteMsg, NftConfigUpdates},
    error::ContractError,
};

use crate::helpers::{assert_err, MockEnv};

pub mod helpers;

#[test]
fn _only_owner_can_update_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let bad_guy = Addr::unchecked("bad_guy");

    // Attempt updating from Rover
    let res = mock.update_nft_config(
        &bad_guy,
        NftConfigUpdates {
            max_value_for_burn: None,
            proposed_new_minter: Some(bad_guy.to_string()),
        },
    );
    assert_err(res, ContractError::OwnerError(NotOwner {}));

    // Attempt updating directly from the NFT contract
    mock.app
        .execute_contract(
            bad_guy.clone(),
            Addr::unchecked(mock.query_config().account_nft.unwrap()),
            &ExecuteMsg::UpdateConfig {
                updates: NftConfigUpdates {
                    max_value_for_burn: None,
                    proposed_new_minter: Some(bad_guy.to_string()),
                },
            },
            &[],
        )
        .unwrap_err();
}

#[test]
fn _raises_on_invalid_config() {
    let mut mock = MockEnv::new().build().unwrap();

    let res = mock.update_nft_config(
        &Addr::unchecked(mock.query_config().owner.unwrap()),
        NftConfigUpdates {
            max_value_for_burn: None,
            proposed_new_minter: Some("".to_string()),
        },
    );

    if res.is_ok() {
        panic!("should have thrown error due to bad proposed_new_minter input")
    }
}

#[test]
fn _update_config_works_with_full_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_nft_config();

    let new_max_value = Some(Uint128::new(1122334455));
    let new_proposed = Some("spiderman_12345".to_string());
    mock.update_nft_config(
        &Addr::unchecked(mock.query_config().owner.unwrap()),
        NftConfigUpdates {
            max_value_for_burn: new_max_value,
            proposed_new_minter: new_proposed.clone(),
        },
    )
    .unwrap();

    let new_config = mock.query_nft_config();
    assert_eq!(Some(new_config.max_value_for_burn), new_max_value);
    assert_eq!(new_config.proposed_new_minter, new_proposed);

    assert_ne!(new_config.max_value_for_burn, original_config.max_value_for_burn);
    assert_ne!(new_config.proposed_new_minter, original_config.proposed_new_minter);
}

#[test]
fn _update_config_works_with_some_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_nft_config();

    let new_proposed = Some("spiderman_12345".to_string());
    mock.update_nft_config(
        &Addr::unchecked(mock.query_config().owner.unwrap()),
        NftConfigUpdates {
            max_value_for_burn: None,
            proposed_new_minter: new_proposed.clone(),
        },
    )
    .unwrap();

    let new_config = mock.query_nft_config();
    assert_eq!(new_config.max_value_for_burn, original_config.max_value_for_burn);
    assert_eq!(new_config.proposed_new_minter, new_proposed);

    assert_ne!(new_config.proposed_new_minter, original_config.proposed_new_minter);
}
