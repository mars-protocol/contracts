extern crate core;

use cosmwasm_std::{Addr, Uint128};
use cw_multi_test::Executor;
use mars_account_nft::{msg::ExecuteMsg, nft_config::NftConfigUpdates};
use mars_owner::OwnerError::NotOwner;
use mars_rover::error::ContractError;

use crate::helpers::{assert_err, MockEnv};

pub mod helpers;

#[test]
fn only_owner_can_update_nft_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let bad_guy = Addr::unchecked("bad_guy");

    // Attempt updating from Rover
    let res = mock.update_nft_config(
        &bad_guy,
        None,
        Some(cw721_base::Action::TransferOwnership {
            new_owner: bad_guy.to_string(),
            expiry: None,
        }),
    );
    assert_err(res, ContractError::Owner(NotOwner {}));

    // Attempt updating directly from the NFT contract
    let account_nft_contract = Addr::unchecked(mock.query_config().account_nft.unwrap());
    mock.app
        .execute_contract(
            bad_guy.clone(),
            account_nft_contract.clone(),
            &ExecuteMsg::UpdateConfig {
                updates: NftConfigUpdates {
                    max_value_for_burn: None,
                    health_contract_addr: None,
                },
            },
            &[],
        )
        .unwrap_err();

    mock.app
        .execute_contract(
            bad_guy.clone(),
            account_nft_contract,
            &ExecuteMsg::UpdateOwnership(cw721_base::Action::TransferOwnership {
                new_owner: bad_guy.to_string(),
                expiry: None,
            }),
            &[],
        )
        .unwrap_err();
}

#[test]
fn raises_on_invalid_config() {
    let mut mock = MockEnv::new().build().unwrap();

    let res = mock.update_nft_config(
        &Addr::unchecked(mock.query_config().ownership.owner.unwrap()),
        None,
        Some(cw721_base::Action::TransferOwnership {
            new_owner: "".to_string(),
            expiry: None,
        }),
    );

    if res.is_ok() {
        panic!("should have thrown error due to bad new_owner input")
    }
}

#[test]
fn update_config_works_with_full_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_nft_config();
    let original_ownership = mock.query_nft_ownership();

    let new_max_value = Some(Uint128::new(1122334455));
    let new_proposed = Some(Addr::unchecked("spiderman_12345"));
    let new_health_contract = Some("new_health_contract_xyz".to_string());

    mock.update_nft_config(
        &Addr::unchecked(mock.query_config().ownership.owner.unwrap()),
        Some(NftConfigUpdates {
            max_value_for_burn: new_max_value,
            health_contract_addr: new_health_contract.clone(),
        }),
        Some(cw721_base::Action::TransferOwnership {
            new_owner: new_proposed.clone().unwrap().into(),
            expiry: None,
        }),
    )
    .unwrap();

    let new_config = mock.query_nft_config();
    assert_eq!(Some(new_config.max_value_for_burn), new_max_value);
    assert_eq!(new_config.health_contract_addr, new_health_contract);

    assert_ne!(new_config.max_value_for_burn, original_config.max_value_for_burn);
    assert_ne!(new_config.health_contract_addr, original_config.health_contract_addr);

    let new_ownership = mock.query_nft_ownership();
    assert_eq!(new_ownership.pending_owner, new_proposed);
    assert_ne!(new_ownership.pending_owner, original_ownership.pending_owner);
}

#[test]
fn update_config_works_with_some_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_nft_config();
    let original_ownership = mock.query_nft_ownership();

    let new_proposed = Some(Addr::unchecked("spiderman_12345"));
    mock.update_nft_config(
        &Addr::unchecked(mock.query_config().ownership.owner.unwrap()),
        None,
        Some(cw721_base::Action::TransferOwnership {
            new_owner: new_proposed.clone().unwrap().into(),
            expiry: None,
        }),
    )
    .unwrap();

    let new_config = mock.query_nft_config();
    assert_eq!(new_config.max_value_for_burn, original_config.max_value_for_burn);
    assert_eq!(new_config.health_contract_addr, original_config.health_contract_addr);

    let new_ownership = mock.query_nft_ownership();
    assert_eq!(new_ownership.pending_owner, new_proposed);
    assert_ne!(new_ownership.pending_owner, original_ownership.pending_owner);
}
