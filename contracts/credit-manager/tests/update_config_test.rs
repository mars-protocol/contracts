use cosmwasm_std::Addr;
use cw721_base::InstantiateMsg as NftInstantiateMsg;
use cw_multi_test::{App, Executor};

use account_nft::msg::ExecuteMsg as NftExecuteMsg;
use rover::adapters::{OracleBase, RedBankBase};
use rover::msg::instantiate::ConfigUpdates;
use rover::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::helpers::{mock_account_nft_contract, mock_app, mock_contract, query_config};

pub mod helpers;

#[test]
fn test_update_config_works_with_full_config() {
    let mut app = mock_app();
    let original_owner = Addr::unchecked("original_owner");
    let code_id = app.store_code(mock_contract());
    let contract_addr = instantiate(&mut app, &original_owner, code_id);

    let original_config = query_config(&app, &contract_addr.clone());
    let original_allowed_vaults = query_allowed_vaults(&mut app, &contract_addr.clone());
    let original_allowed_assets = query_allowed_assets(&mut app, &contract_addr.clone());

    let nft_contract_addr = setup_nft_and_propose_owner(&mut app, &original_owner, &contract_addr);
    let new_owner = Addr::unchecked("new_owner");
    let new_red_bank = RedBankBase::new("new_red_bank".to_string());
    let new_allowed_vaults = vec!["vaultcontract1".to_string()];
    let new_allowed_assets = vec!["uosmo".to_string()];
    let new_oracle = OracleBase::new("new_oracle".to_string());

    app.execute_contract(
        original_owner.clone(),
        contract_addr.clone(),
        &ExecuteMsg::UpdateConfig {
            new_config: ConfigUpdates {
                account_nft: Some(nft_contract_addr.to_string()),
                owner: Some(new_owner.to_string()),
                allowed_vaults: Some(new_allowed_vaults.clone()),
                allowed_coins: Some(new_allowed_assets.clone()),
                red_bank: Some(new_red_bank.clone()),
                oracle: Some(new_oracle.clone()),
            },
        },
        &[],
    )
    .unwrap();

    let new_config = query_config(&app, &contract_addr.clone());
    let new_queried_allowed_vaults = query_allowed_vaults(&mut app, &contract_addr.clone());
    let new_queried_allowed_assets = query_allowed_assets(&mut app, &contract_addr.clone());

    assert_eq!(new_config.account_nft, Some(nft_contract_addr.to_string()));
    assert_ne!(new_config.account_nft, original_config.account_nft);

    assert_eq!(new_config.owner, new_owner.to_string());
    assert_ne!(new_config.owner, original_config.owner);

    assert_eq!(new_queried_allowed_vaults, new_allowed_vaults);
    assert_ne!(new_queried_allowed_vaults, original_allowed_vaults);

    assert_eq!(new_queried_allowed_assets, new_allowed_assets);
    assert_ne!(new_queried_allowed_assets, original_allowed_assets);

    assert_eq!(&new_config.red_bank, new_red_bank.address());
    assert_ne!(new_config.red_bank, original_config.red_bank);

    assert_eq!(&new_config.oracle, new_oracle.address());
    assert_ne!(new_config.oracle, original_config.oracle);
}

#[test]
fn test_update_config_works_with_some_config() {
    let mut app = mock_app();
    let original_owner = Addr::unchecked("original_owner");
    let code_id = app.store_code(mock_contract());
    let contract_addr = instantiate(&mut app, &original_owner, code_id);

    let original_config = query_config(&app, &contract_addr.clone());
    let original_allowed_vaults = query_allowed_vaults(&mut app, &contract_addr.clone());
    let original_allowed_assets = query_allowed_assets(&mut app, &contract_addr.clone());

    let nft_contract_addr = setup_nft_and_propose_owner(&mut app, &original_owner, &contract_addr);
    let new_allowed_vaults = vec!["vaultcontract1".to_string()];

    app.execute_contract(
        original_owner.clone(),
        contract_addr.clone(),
        &ExecuteMsg::UpdateConfig {
            new_config: ConfigUpdates {
                account_nft: Some(nft_contract_addr.to_string()),
                allowed_vaults: Some(new_allowed_vaults.clone()),
                ..Default::default()
            },
        },
        &[],
    )
    .unwrap();

    let new_config = query_config(&app, &contract_addr.clone());
    let new_queried_allowed_vaults = query_allowed_vaults(&mut app, &contract_addr.clone());
    let new_queried_allowed_assets = query_allowed_assets(&mut app, &contract_addr.clone());

    // Changed configs
    assert_eq!(new_config.account_nft, Some(nft_contract_addr.to_string()));
    assert_ne!(new_config.account_nft, original_config.account_nft);

    assert_eq!(new_queried_allowed_vaults, new_allowed_vaults);
    assert_ne!(new_queried_allowed_vaults, original_allowed_vaults);

    // Unchanged configs
    assert_eq!(new_config.owner, original_config.owner);
    assert_eq!(original_allowed_assets, new_queried_allowed_assets);
    assert_eq!(new_config.red_bank, original_config.red_bank);
}

#[test]
fn test_update_config_does_nothing_when_nothing_is_passed() {
    let mut app = mock_app();
    let original_owner = Addr::unchecked("original_owner");
    let code_id = app.store_code(mock_contract());
    let contract_addr = instantiate(&mut app, &original_owner, code_id);

    let original_config = query_config(&app, &contract_addr);
    let original_allowed_vaults = query_allowed_vaults(&mut app, &contract_addr);
    let original_allowed_assets = query_allowed_assets(&mut app, &contract_addr);

    app.execute_contract(
        original_owner.clone(),
        contract_addr.clone(),
        &ExecuteMsg::UpdateConfig {
            new_config: Default::default(),
        },
        &[],
    )
    .unwrap();

    let new_config = query_config(&app, &contract_addr);
    let new_queried_allowed_vaults = query_allowed_vaults(&mut app, &contract_addr);
    let new_queried_allowed_assets = query_allowed_assets(&mut app, &contract_addr);

    assert_eq!(new_config.account_nft, original_config.account_nft);
    assert_eq!(new_config.owner, original_config.owner);
    assert_eq!(new_queried_allowed_vaults, original_allowed_vaults);
    assert_eq!(new_queried_allowed_assets, original_allowed_assets);
    assert_eq!(new_config.red_bank, original_config.red_bank);
}

fn instantiate(app: &mut App, original_owner: &Addr, code_id: u64) -> Addr {
    app.instantiate_contract(
        code_id,
        original_owner.clone(),
        &InstantiateMsg {
            owner: original_owner.to_string(),
            allowed_vaults: vec![],
            allowed_coins: vec![],
            red_bank: RedBankBase::new("initial_red_bank".to_string()),
            oracle: OracleBase::new("initial_oracle".to_string()),
        },
        &[],
        "mock_manager_contract",
        None,
    )
    .unwrap()
}

fn setup_nft_and_propose_owner(app: &mut App, original_owner: &Addr, contract_addr: &Addr) -> Addr {
    let nft_contract_code_id = app.store_code(mock_account_nft_contract());
    let nft_contract_addr = app
        .instantiate_contract(
            nft_contract_code_id,
            original_owner.clone(),
            &NftInstantiateMsg {
                name: "Rover Credit Account".to_string(),
                symbol: "RCA".to_string(),
                minter: original_owner.to_string(),
            },
            &[],
            "manager-mock-account-nft",
            None,
        )
        .unwrap();

    let proposal_msg: NftExecuteMsg = NftExecuteMsg::ProposeNewOwner {
        new_owner: contract_addr.to_string(),
    };
    app.execute_contract(
        original_owner.clone(),
        nft_contract_addr.clone(),
        &proposal_msg,
        &[],
    )
    .unwrap();
    nft_contract_addr
}

fn query_allowed_vaults(app: &mut App, contract_addr: &Addr) -> Vec<String> {
    app.wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedVaults {
                start_after: None,
                limit: None,
            },
        )
        .unwrap()
}

fn query_allowed_assets(app: &mut App, contract_addr: &Addr) -> Vec<String> {
    app.wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedCoins {
                start_after: None,
                limit: None,
            },
        )
        .unwrap()
}
