use cosmwasm_std::Addr;
use cw_asset::AssetInfoUnchecked;
use cw_multi_test::Executor;

use rover::{InstantiateMsg, QueryMsg};

use crate::helpers::{mock_account_nft_contract, mock_app, mock_contract};

mod helpers;

#[test]
fn test_owner_set_on_instantiate() {
    let mut app = mock_app();
    let nft_contract_code_id = app.store_code(mock_account_nft_contract());
    let manager_code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec![],
        allowed_assets: vec![],
        nft_contract_code_id,
    };

    let contract_addr = app
        .instantiate_contract(
            manager_code_id,
            owner.clone(),
            &msg,
            &[],
            "mock-account-nft",
            None,
        )
        .unwrap();

    let res: String = app
        .wrap()
        .query_wasm_smart(contract_addr.clone(), &QueryMsg::Owner {})
        .unwrap();

    assert_eq!(owner, res);
}

#[test]
fn test_nft_contract_addr_set_on_instantiate() {
    let mut app = mock_app();
    let owner = Addr::unchecked("owner");

    let nft_contract_code_id = app.store_code(mock_account_nft_contract());

    let credit_manager_code_id = app.store_code(mock_contract());
    let manager_initiate_msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec![],
        allowed_assets: vec![],
        nft_contract_code_id,
    };

    let contract_addr = app
        .instantiate_contract(
            credit_manager_code_id,
            owner.clone(),
            &manager_initiate_msg,
            &[],
            "manager-mock-account-nft",
            None,
        )
        .unwrap();

    let res: String = app
        .wrap()
        .query_wasm_smart(contract_addr.clone(), &QueryMsg::CreditAccountNftAddress {})
        .unwrap();

    assert!(res.as_str().chars().count() > 0);
}

#[test]
fn test_allowed_vaults_and_assets_stored_on_instantiate() {
    let mut app = mock_app();
    let nft_contract_code_id = app.store_code(mock_account_nft_contract());
    let credit_manager_code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");

    let allowed_vaults = vec![
        String::from("vaultcontract1"),
        String::from("vaultcontract2"),
        String::from("vaultcontract3"),
    ];

    let allowed_assets = vec![
        AssetInfoUnchecked::Native(String::from("uosmo")),
        AssetInfoUnchecked::Cw20(String::from("osmo85wwjycfxjlaxsae9asmxlk3bsgxbw")),
        AssetInfoUnchecked::Cw20(String::from("osmompbtkt3jezatztteo577lxkqbkdyke")),
        AssetInfoUnchecked::Cw20(String::from("osmos6kmpxz9xcstleqnu2fnz8gskgf6gx")),
    ];

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: allowed_vaults.clone(),
        allowed_assets: allowed_assets.clone(),
        nft_contract_code_id,
    };

    let contract_addr = app
        .instantiate_contract(
            credit_manager_code_id,
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
    let nft_contract_code_id = app.store_code(mock_account_nft_contract());
    let manager_code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec![String::from("%%%INVALID%%%")],
        allowed_assets: vec![],
        nft_contract_code_id,
    };

    let instantiate_res = app.instantiate_contract(
        manager_code_id,
        owner.clone(),
        &msg,
        &[],
        "mock-contract",
        None,
    );

    match instantiate_res {
        Err(_) => {}
        Ok(_) => panic!("Should have thrown an error"),
    }

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec![],
        allowed_assets: vec![AssetInfoUnchecked::Cw20(String::from("AA"))], // Because cw-asset lowercases before passing to validate, in the test env, two letter strings is only one that triggers a fail
        nft_contract_code_id,
    };

    let instantiate_res = app.instantiate_contract(
        manager_code_id,
        owner.clone(),
        &msg,
        &[],
        "mock-contract",
        None,
    );

    match instantiate_res {
        Err(_) => {}
        Ok(_) => panic!("Should have thrown an error"),
    }

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec![],
        allowed_assets: vec![],
        nft_contract_code_id: 0,
    };

    let instantiate_res =
        app.instantiate_contract(manager_code_id, owner, &msg, &[], "mock-contract", None);

    match instantiate_res {
        Err(_) => {}
        Ok(_) => panic!("Should have thrown an error"),
    }
}
