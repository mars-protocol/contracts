use account_nft::msg::ExecuteMsg as NftExecuteMsg;
use cosmwasm_std::Addr;
use cw721_base::InstantiateMsg as NftInstantiateMsg;
use cw_multi_test::{App, Executor};

use rover::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::helpers::{mock_account_nft_contract, mock_app, mock_contract};

pub mod helpers;

#[test]
fn test_update_config_works_with_full_config() {
    let mut app = mock_app();
    let original_owner = Addr::unchecked("original_owner");
    let code_id = app.store_code(mock_contract());
    let contract_addr = instantiate(&mut app, &original_owner, code_id);

    let config_res = query_config(&mut app, &contract_addr.clone());

    assert_eq!(config_res.account_nft, None);
    assert_eq!(config_res.owner, original_owner.to_string());

    let new_owner = Addr::unchecked("new_owner");

    let account_nft_contract = setup_nft_contract(&mut app, &original_owner, &contract_addr);

    app.execute_contract(
        original_owner.clone(),
        contract_addr.clone(),
        &ExecuteMsg::UpdateConfig {
            account_nft: Some(account_nft_contract.to_string()),
            owner: Some(new_owner.to_string()),
        },
        &[],
    )
    .unwrap();

    let config_res = query_config(&mut app, &contract_addr.clone());

    assert_eq!(
        config_res.account_nft,
        Some(account_nft_contract.to_string())
    );
    assert_eq!(config_res.owner, new_owner.to_string());
}

#[test]
fn test_update_config_works_with_some_config() {
    let mut app = mock_app();
    let original_owner = Addr::unchecked("original_owner");
    let code_id = app.store_code(mock_contract());
    let contract_addr = instantiate(&mut app, &original_owner, code_id);

    let config_res = query_config(&mut app, &contract_addr.clone());

    assert_eq!(config_res.account_nft, None);
    assert_eq!(config_res.owner, original_owner.to_string());

    let account_nft_contract = setup_nft_contract(&mut app, &original_owner, &contract_addr);
    app.execute_contract(
        original_owner.clone(),
        contract_addr.clone(),
        &ExecuteMsg::UpdateConfig {
            account_nft: Some(account_nft_contract.to_string()),
            owner: None,
        },
        &[],
    )
    .unwrap();

    let config_res = query_config(&mut app, &contract_addr.clone());

    assert_eq!(
        config_res.account_nft,
        Some(account_nft_contract.to_string())
    );
    assert_eq!(config_res.owner, original_owner.to_string());

    let new_owner = Addr::unchecked("new_owner");
    app.execute_contract(
        original_owner.clone(),
        contract_addr.clone(),
        &ExecuteMsg::UpdateConfig {
            account_nft: None,
            owner: Some(new_owner.to_string()),
        },
        &[],
    )
    .unwrap();

    let config_res = query_config(&mut app, &contract_addr.clone());
    assert_eq!(
        config_res.account_nft,
        Some(account_nft_contract.to_string())
    );
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
        &ExecuteMsg::UpdateConfig {
            account_nft: None,
            owner: None,
        },
        &[],
    )
    .unwrap();

    let config_res = query_config(&mut app, &contract_addr.clone());

    assert_eq!(config_res.account_nft, None);
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

fn setup_nft_contract(app: &mut App, owner: &Addr, contract: &Addr) -> Addr {
    let nft_contract_code_id = app.store_code(mock_account_nft_contract());
    let nft_contract_addr = app
        .instantiate_contract(
            nft_contract_code_id,
            owner.clone(),
            &NftInstantiateMsg {
                name: String::from("Rover Credit Account"),
                symbol: String::from("RCA"),
                minter: owner.to_string(),
            },
            &[],
            "manager-mock-account-nft",
            None,
        )
        .unwrap();

    let proposal_msg: NftExecuteMsg = NftExecuteMsg::ProposeNewOwner {
        new_owner: contract.to_string(),
    };
    app.execute_contract(owner.clone(), nft_contract_addr.clone(), &proposal_msg, &[])
        .unwrap();
    nft_contract_addr
}
