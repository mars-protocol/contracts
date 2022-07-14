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
        "addr1".to_string(),
        "addr2".to_string(),
        "addr3".to_string(),
        "addr4".to_string(),
        "addr5".to_string(),
        "addr6".to_string(),
        "addr7".to_string(),
        "addr8".to_string(),
        "addr9".to_string(),
        "addr10".to_string(),
        "addr11".to_string(),
        "addr12".to_string(),
        "addr13".to_string(),
        "addr14".to_string(),
        "addr15".to_string(),
        "addr16".to_string(),
        "addr17".to_string(),
        "addr18".to_string(),
        "addr19".to_string(),
        "addr20".to_string(),
        "addr21".to_string(),
        "addr22".to_string(),
        "addr23".to_string(),
        "addr24".to_string(),
        "addr25".to_string(),
        "addr26".to_string(),
        "addr27".to_string(),
        "addr28".to_string(),
        "addr29".to_string(),
        "addr30".to_string(),
        "addr31".to_string(),
        "addr32".to_string(),
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
        AssetInfoUnchecked::Native("native_asset_1".to_string()),
        AssetInfoUnchecked::Native("native_asset_2".to_string()),
        AssetInfoUnchecked::Native("native_asset_3".to_string()),
        AssetInfoUnchecked::Native("native_asset_4".to_string()),
        AssetInfoUnchecked::Native("native_asset_5".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_1".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_2".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_3".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_4".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_5".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_6".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_7".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_8".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_9".to_string()),
        AssetInfoUnchecked::Native("native_asset_6".to_string()),
        AssetInfoUnchecked::Native("native_asset_7".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_10".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_11".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_12".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_13".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_14".to_string()),
        AssetInfoUnchecked::Native("native_asset_8".to_string()),
        AssetInfoUnchecked::Native("native_asset_9".to_string()),
        AssetInfoUnchecked::Native("native_asset_10".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_15".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_16".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_17".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_18".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_19".to_string()),
        AssetInfoUnchecked::Cw20("cw_token_20".to_string()),
        AssetInfoUnchecked::Native("native_asset_11".to_string()),
        AssetInfoUnchecked::Native("native_asset_12".to_string()),
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
