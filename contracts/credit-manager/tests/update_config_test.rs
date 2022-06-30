extern crate core;

use cosmwasm_std::Addr;
use cw_multi_test::{App, Executor};
use rover::ExecuteMsg::UpdateConfig;

use crate::helpers::{mock_app, mock_contract};
use rover::{ConfigResponse, InstantiateMsg, QueryMsg};

pub mod helpers;

#[test]
fn test_update_config_works_with_full_config() {
    let mut app = mock_app();
    let original_owner = Addr::unchecked("original_owner");
    let code_id = app.store_code(mock_contract());
    let contract_addr = instantiate(&mut app, &original_owner, code_id);

    let config_res = query_config(&mut app, &contract_addr.clone());

    assert_eq!(config_res.account_nft, "");
    assert_eq!(config_res.owner, original_owner.to_string());

    let new_owner = Addr::unchecked("new_owner");
    let account_nft_contract = Addr::unchecked("account_nft_contract");
    app.execute_contract(
        original_owner.clone(),
        contract_addr.clone(),
        &UpdateConfig {
            account_nft: Some(account_nft_contract.to_string()),
            owner: Some(new_owner.to_string()),
        },
        &[],
    )
    .unwrap();

    let config_res = query_config(&mut app, &contract_addr.clone());

    assert_eq!(config_res.account_nft, account_nft_contract.to_string());
    assert_eq!(config_res.owner, new_owner.to_string());
}

#[test]
fn test_update_config_works_with_some_config() {
    let mut app = mock_app();
    let original_owner = Addr::unchecked("original_owner");
    let code_id = app.store_code(mock_contract());
    let contract_addr = instantiate(&mut app, &original_owner, code_id);

    let config_res = query_config(&mut app, &contract_addr.clone());

    assert_eq!(config_res.account_nft, "");
    assert_eq!(config_res.owner, original_owner.to_string());

    let account_nft_contract = Addr::unchecked("account_nft_contract");
    app.execute_contract(
        original_owner.clone(),
        contract_addr.clone(),
        &UpdateConfig {
            account_nft: Some(account_nft_contract.to_string()),
            owner: None,
        },
        &[],
    )
    .unwrap();

    let config_res = query_config(&mut app, &contract_addr.clone());

    assert_eq!(config_res.account_nft, account_nft_contract.to_string());
    assert_eq!(config_res.owner, original_owner.to_string());

    let new_owner = Addr::unchecked("new_owner");
    app.execute_contract(
        original_owner.clone(),
        contract_addr.clone(),
        &UpdateConfig {
            account_nft: None,
            owner: Some(new_owner.to_string()),
        },
        &[],
    )
    .unwrap();

    let config_res = query_config(&mut app, &contract_addr.clone());
    assert_eq!(config_res.account_nft, account_nft_contract.to_string());
    assert_eq!(config_res.owner, new_owner.to_string());
}

#[test]
fn test_update_config_does_nothing_when_nothing_is_passed() {
    let mut app = mock_app();
    let original_owner = Addr::unchecked("original_owner");
    let code_id = app.store_code(mock_contract());
    let contract_addr = instantiate(&mut app, &original_owner, code_id);

    app.execute_contract(
        original_owner.clone(),
        contract_addr.clone(),
        &UpdateConfig {
            account_nft: None,
            owner: None,
        },
        &[],
    )
    .unwrap();

    let config_res = query_config(&mut app, &contract_addr.clone());

    assert_eq!(config_res.account_nft, "");
    assert_eq!(config_res.owner, original_owner.to_string());
}

fn query_config(app: &mut App, contract_addr: &Addr) -> ConfigResponse {
    app.wrap()
        .query_wasm_smart(contract_addr.clone(), &QueryMsg::Config {})
        .unwrap()
}

fn instantiate(app: &mut App, original_owner: &Addr, code_id: u64) -> Addr {
    app.instantiate_contract(
        code_id,
        original_owner.clone(),
        &InstantiateMsg {
            owner: original_owner.to_string(),
            allowed_vaults: vec![],
            allowed_assets: vec![],
        },
        &[],
        "mock_manager_contract",
        None,
    )
    .unwrap()
}
