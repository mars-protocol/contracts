use cosmwasm_std::Addr;
use cw_asset::AssetInfoUnchecked;
use cw_multi_test::Executor;

use rover::msg::{InstantiateMsg, QueryMsg};

use crate::helpers::{mock_app, mock_contract};

pub mod helpers;

#[test]
fn test_pagination_on_allowed_vaults_query_works() {
    let mut app = mock_app();
    let code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");

    let allowed_vaults = vec![
        String::from("addr1"),
        String::from("addr2"),
        String::from("addr3"),
        String::from("addr4"),
        String::from("addr5"),
        String::from("addr6"),
        String::from("addr7"),
        String::from("addr8"),
        String::from("addr9"),
        String::from("addr10"),
        String::from("addr11"),
        String::from("addr12"),
        String::from("addr13"),
        String::from("addr14"),
        String::from("addr15"),
        String::from("addr16"),
        String::from("addr17"),
        String::from("addr18"),
        String::from("addr19"),
        String::from("addr20"),
        String::from("addr21"),
        String::from("addr22"),
        String::from("addr23"),
        String::from("addr24"),
        String::from("addr25"),
        String::from("addr26"),
        String::from("addr27"),
        String::from("addr28"),
        String::from("addr29"),
        String::from("addr30"),
        String::from("addr31"),
        String::from("addr32"),
    ];

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: allowed_vaults.clone(),
        allowed_assets: vec![],
    };

    let contract_addr = app
        .instantiate_contract(code_id, owner.clone(), &msg, &[], "mock-contract", None)
        .unwrap();

    let vaults_res: Vec<String> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedVaults {
                start_after: None,
                limit: Some(58 as u32),
            },
        )
        .unwrap();

    // Assert maximum is observed
    assert_eq!(vaults_res.len(), 30);

    let vaults_res: Vec<String> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedVaults {
                start_after: None,
                limit: Some(2 as u32),
            },
        )
        .unwrap();

    // Assert limit request is observed
    assert_eq!(vaults_res.len(), 2);

    let vaults_res_a: Vec<String> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedVaults {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    let vaults_res_b: Vec<String> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedVaults {
                start_after: Some(vaults_res_a.last().unwrap().clone()),
                limit: None,
            },
        )
        .unwrap();

    let vaults_res_c: Vec<String> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedVaults {
                start_after: Some(vaults_res_b.last().unwrap().clone()),
                limit: None,
            },
        )
        .unwrap();

    let vaults_res_d: Vec<String> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedVaults {
                start_after: Some(vaults_res_c.last().unwrap().clone()),
                limit: None,
            },
        )
        .unwrap();

    // Assert default is observed
    assert_eq!(vaults_res_a.len(), 10);
    assert_eq!(vaults_res_b.len(), 10);
    assert_eq!(vaults_res_c.len(), 10);

    assert_eq!(vaults_res_d.len(), 2);

    let combined: Vec<String> = vaults_res_a
        .iter()
        .cloned()
        .chain(vaults_res_b.iter().cloned())
        .chain(vaults_res_c.iter().cloned())
        .chain(vaults_res_d.iter().cloned())
        .collect();

    assert_eq!(combined.len(), allowed_vaults.len());
    assert!(allowed_vaults.iter().all(|item| combined.contains(item)));
}

#[test]
fn test_pagination_on_allowed_assets_query_works() {
    let mut app = mock_app();
    let code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");

    let allowed_assets = vec![
        AssetInfoUnchecked::Native(String::from("native_asset_1")),
        AssetInfoUnchecked::Native(String::from("native_asset_2")),
        AssetInfoUnchecked::Native(String::from("native_asset_3")),
        AssetInfoUnchecked::Native(String::from("native_asset_4")),
        AssetInfoUnchecked::Native(String::from("native_asset_5")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_1")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_2")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_3")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_4")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_5")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_6")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_7")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_8")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_9")),
        AssetInfoUnchecked::Native(String::from("native_asset_6")),
        AssetInfoUnchecked::Native(String::from("native_asset_7")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_10")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_11")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_12")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_13")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_14")),
        AssetInfoUnchecked::Native(String::from("native_asset_8")),
        AssetInfoUnchecked::Native(String::from("native_asset_9")),
        AssetInfoUnchecked::Native(String::from("native_asset_10")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_15")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_16")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_17")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_18")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_19")),
        AssetInfoUnchecked::Cw20(String::from("cw_token_20")),
        AssetInfoUnchecked::Native(String::from("native_asset_11")),
        AssetInfoUnchecked::Native(String::from("native_asset_12")),
    ];

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec![],
        allowed_assets: allowed_assets.clone(),
    };

    let contract_addr = app
        .instantiate_contract(code_id, owner.clone(), &msg, &[], "mock-contract", None)
        .unwrap();

    let assets_res: Vec<AssetInfoUnchecked> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedAssets {
                start_after: None,
                limit: Some(58 as u32),
            },
        )
        .unwrap();

    // Assert maximum is observed
    assert_eq!(assets_res.len(), 30);

    let assets_res: Vec<AssetInfoUnchecked> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedAssets {
                start_after: None,
                limit: Some(2 as u32),
            },
        )
        .unwrap();

    // Assert limit request is observed
    assert_eq!(assets_res.len(), 2);

    let assets_res_a: Vec<AssetInfoUnchecked> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedAssets {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    let assets_res_b: Vec<AssetInfoUnchecked> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedAssets {
                start_after: Some(assets_res_a.last().unwrap().clone()),
                limit: None,
            },
        )
        .unwrap();

    let assets_res_c: Vec<AssetInfoUnchecked> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedAssets {
                start_after: Some(assets_res_b.last().unwrap().clone()),
                limit: None,
            },
        )
        .unwrap();

    let assets_res_d: Vec<AssetInfoUnchecked> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedAssets {
                start_after: Some(assets_res_c.last().unwrap().clone()),
                limit: None,
            },
        )
        .unwrap();

    // Assert default is observed
    assert_eq!(assets_res_a.len(), 10);
    assert_eq!(assets_res_b.len(), 10);
    assert_eq!(assets_res_c.len(), 10);

    assert_eq!(assets_res_d.len(), 2);

    let combined: Vec<AssetInfoUnchecked> = assets_res_a
        .iter()
        .cloned()
        .chain(assets_res_b.iter().cloned())
        .chain(assets_res_c.iter().cloned())
        .chain(assets_res_d.iter().cloned())
        .collect();

    assert_eq!(combined.len(), allowed_assets.len());
    assert!(allowed_assets.iter().all(|item| combined.contains(item)));
}
