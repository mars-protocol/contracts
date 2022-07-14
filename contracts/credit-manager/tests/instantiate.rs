use cosmwasm_std::Addr;
use cw_asset::AssetInfoUnchecked;
use cw_multi_test::Executor;

use rover::msg::query::{ConfigResponse, QueryMsg};
use rover::msg::InstantiateMsg;

use crate::helpers::{mock_app, mock_contract};

pub mod helpers;

#[test]
fn test_owner_set_on_instantiate() {
    let mut app = mock_app();
    let code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec![],
        allowed_assets: vec![],
    };

    let contract_addr = app
        .instantiate_contract(code_id, owner.clone(), &msg, &[], "mock-account-nft", None)
        .unwrap();

    let res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.clone(), &QueryMsg::Config {})
        .unwrap();

    assert_eq!(owner, res.owner);
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
                allowed_assets: vec![],
            },
            &[],
            "manager-mock-account-nft",
            None,
        )
        .unwrap();

    let res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.clone(), &QueryMsg::Config {})
        .unwrap();

    assert_eq!(res.account_nft, None);
}

#[test]
fn test_allowed_vaults_and_assets_stored_on_instantiate() {
    let mut app = mock_app();
    let code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");

    let allowed_vaults = vec![
        "vaultcontract1".to_string(),
        "vaultcontract2".to_string(),
        "vaultcontract3".to_string(),
    ];

    let allowed_assets = vec![
        AssetInfoUnchecked::Native("uosmo".to_string()),
        AssetInfoUnchecked::Cw20("osmo85wwjycfxjlaxsae9asmxlk3bsgxbw".to_string()),
        AssetInfoUnchecked::Cw20("osmompbtkt3jezatztteo577lxkqbkdyke".to_string()),
        AssetInfoUnchecked::Cw20("osmos6kmpxz9xcstleqnu2fnz8gskgf6gx".to_string()),
    ];

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: allowed_vaults.clone(),
        allowed_assets: allowed_assets.clone(),
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

    let assets_res: Vec<AssetInfoUnchecked> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedAssets {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(assets_res.len(), 4);
    assert!(allowed_assets.iter().all(|item| assets_res.contains(item)));

    let vaults_res: Vec<String> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedVaults {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(vaults_res.len(), 3);
    assert_eq!(allowed_vaults, vaults_res);
}

#[test]
fn test_panics_on_invalid_instantiation_addrs() {
    let mut app = mock_app();
    let manager_code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec!["%%%INVALID%%%".to_string()],
        allowed_assets: vec![],
    };

    let instantiate_res = app.instantiate_contract(
        manager_code_id,
        owner.clone(),
        &msg,
        &[],
        "mock-contract",
        None,
    );

    if instantiate_res.is_ok() {
        panic!("Should have thrown an error");
    }

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec![],
        allowed_assets: vec![AssetInfoUnchecked::Cw20("AA".to_string())], // Because cw-asset lowercases before passing to validate, in the test env, two letter strings is only one that triggers a fail
    };

    let instantiate_res = app.instantiate_contract(
        manager_code_id,
        owner.clone(),
        &msg,
        &[],
        "mock-contract",
        None,
    );

    if instantiate_res.is_ok() {
        panic!("Should have thrown an error");
    }
}
