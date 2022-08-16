use cosmwasm_std::Addr;
use cw_multi_test::Executor;

use rover::adapters::{OracleBase, RedBankBase};
use rover::msg::query::{ConfigResponse, QueryMsg};
use rover::msg::InstantiateMsg;

use crate::helpers::{assert_contents_equal, mock_app, mock_contract};

pub mod helpers;

#[test]
fn test_owner_set_on_instantiate() {
    let mut app = mock_app();
    let code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec![],
        allowed_coins: vec![],
        red_bank: RedBankBase::new("red_bank_contract".to_string()),
        oracle: OracleBase::new("oracle_contract".to_string()),
    };

    let contract_addr = app
        .instantiate_contract(code_id, owner.clone(), &msg, &[], "mock-account-nft", None)
        .unwrap();

    let res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::Config {})
        .unwrap();

    assert_eq!(owner, res.owner);
}

#[test]
fn test_raises_on_invalid_owner_addr() {
    let mut app = mock_app();
    let manager_code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("%%%INVALID%%%");

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec![],
        allowed_coins: vec![],
        red_bank: RedBankBase::new("red_bank_contract".to_string()),
        oracle: OracleBase::new("oracle_contract".to_string()),
    };

    let instantiate_res =
        app.instantiate_contract(manager_code_id, owner, &msg, &[], "mock-contract", None);

    if instantiate_res.is_ok() {
        panic!("Should have thrown an error");
    }
}

#[test]
fn test_nft_contract_addr_not_set_on_instantiate() {
    let mut app = mock_app();
    let owner = Addr::unchecked("owner");
    let code_id = app.store_code(mock_contract());

    let contract_addr = app
        .instantiate_contract(
            code_id,
            owner.clone(),
            &InstantiateMsg {
                owner: owner.to_string(),
                allowed_vaults: vec![],
                allowed_coins: vec![],
                red_bank: RedBankBase::new("red_bank_contract".to_string()),
                oracle: OracleBase::new("oracle_contract".to_string()),
            },
            &[],
            "manager-mock-account-nft",
            None,
        )
        .unwrap();

    let res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::Config {})
        .unwrap();

    assert_eq!(res.account_nft, None);
}

#[test]
fn test_allowed_vaults_set_on_instantiate() {
    let mut app = mock_app();
    let code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");

    let allowed_vaults = vec![
        "vaultcontract1".to_string(),
        "vaultcontract2".to_string(),
        "vaultcontract3".to_string(),
    ];

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: allowed_vaults.clone(),
        allowed_coins: vec![],
        red_bank: RedBankBase::new("red_bank_contract".to_string()),
        oracle: OracleBase::new("oracle_contract".to_string()),
    };

    let contract_addr = app
        .instantiate_contract(
            code_id,
            owner,
            &msg,
            &[],
            "mock-credit-manager-contract",
            None,
        )
        .unwrap();

    let vaults_res: Vec<String> = app
        .wrap()
        .query_wasm_smart(
            contract_addr,
            &QueryMsg::AllowedVaults {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_contents_equal(vaults_res, allowed_vaults);
}

#[test]
fn test_raises_on_invalid_vaults_addr() {
    let mut app = mock_app();
    let manager_code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec!["%%%INVALID%%%".to_string()],
        allowed_coins: vec![],
        red_bank: RedBankBase::new("red_bank_contract".to_string()),
        oracle: OracleBase::new("oracle_contract".to_string()),
    };

    let instantiate_res =
        app.instantiate_contract(manager_code_id, owner, &msg, &[], "mock-contract", None);

    if instantiate_res.is_ok() {
        panic!("Should have thrown an error");
    }
}

#[test]
fn test_allowed_coins_set_on_instantiate() {
    let mut app = mock_app();
    let code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");

    let allowed_coins = vec![
        "uosmo".to_string(),
        "uatom".to_string(),
        "umars".to_string(),
        "ujake".to_string(),
    ];

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec![],
        allowed_coins: allowed_coins.clone(),
        red_bank: RedBankBase::new("red_bank_contract".to_string()),
        oracle: OracleBase::new("oracle_contract".to_string()),
    };

    let contract_addr = app
        .instantiate_contract(
            code_id,
            owner,
            &msg,
            &[],
            "mock-credit-manager-contract",
            None,
        )
        .unwrap();

    let coins_res: Vec<String> = app
        .wrap()
        .query_wasm_smart(
            contract_addr,
            &QueryMsg::AllowedCoins {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_contents_equal(coins_res, allowed_coins)
}

#[test]
fn test_red_bank_set_on_instantiate() {
    let mut app = mock_app();
    let code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");
    let red_bank_addr = "red_bank_contract".to_string();

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec![],
        allowed_coins: vec![],
        red_bank: RedBankBase::new("red_bank_contract".to_string()),
        oracle: OracleBase::new("oracle_contract".to_string()),
    };

    let contract_addr = app
        .instantiate_contract(code_id, owner, &msg, &[], "mock-account-nft", None)
        .unwrap();

    let res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::Config {})
        .unwrap();

    assert_eq!(red_bank_addr, res.red_bank);
}

#[test]
fn test_raises_on_invalid_red_bank_addr() {
    let mut app = mock_app();
    let manager_code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_coins: vec![],
        allowed_vaults: vec![],
        red_bank: RedBankBase::new("%%%INVALID%%%".to_string()),
        oracle: OracleBase::new("oracle_contract".to_string()),
    };

    let instantiate_res =
        app.instantiate_contract(manager_code_id, owner, &msg, &[], "mock-contract", None);

    if instantiate_res.is_ok() {
        panic!("Should have thrown an error");
    }
}

#[test]
fn test_oracle_set_on_instantiate() {
    let mut app = mock_app();
    let code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");
    let oracle_contract = "oracle_contract".to_string();

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_coins: vec![],
        allowed_vaults: vec![],
        red_bank: RedBankBase::new("red_bank_contract".to_string()),
        oracle: OracleBase::new("oracle_contract".to_string()),
    };

    let contract_addr = app
        .instantiate_contract(code_id, owner, &msg, &[], "mock-account-nft", None)
        .unwrap();

    let res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::Config {})
        .unwrap();

    assert_eq!(oracle_contract, res.oracle);
}

#[test]
fn test_raises_on_invalid_oracle_addr() {
    let mut app = mock_app();
    let manager_code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec![],
        allowed_coins: vec![],
        red_bank: RedBankBase::new("red_bank_contract".to_string()),
        oracle: OracleBase::new("%%%INVALID%%%".to_string()),
    };

    let instantiate_res =
        app.instantiate_contract(manager_code_id, owner, &msg, &[], "mock-contract", None);

    if instantiate_res.is_ok() {
        panic!("Should have thrown an error");
    }
}
