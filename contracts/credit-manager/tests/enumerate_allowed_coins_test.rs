use cosmwasm_std::Addr;
use cw_multi_test::Executor;

use rover::adapters::{OracleBase, RedBankBase};
use rover::msg::{InstantiateMsg, QueryMsg};

use crate::helpers::{mock_app, mock_contract};

pub mod helpers;

#[test]
fn test_pagination_on_allowed_coins_query_works() {
    let mut app = mock_app();
    let code_id = app.store_code(mock_contract());
    let owner = Addr::unchecked("owner");

    let allowed_coins = vec![
        "coin_1".to_string(),
        "coin_2".to_string(),
        "coin_3".to_string(),
        "coin_4".to_string(),
        "coin_5".to_string(),
        "coin_6".to_string(),
        "coin_7".to_string(),
        "coin_8".to_string(),
        "coin_9".to_string(),
        "coin_10".to_string(),
        "coin_11".to_string(),
        "coin_12".to_string(),
        "coin_13".to_string(),
        "coin_14".to_string(),
        "coin_15".to_string(),
        "coin_16".to_string(),
        "coin_17".to_string(),
        "coin_18".to_string(),
        "coin_19".to_string(),
        "coin_20".to_string(),
        "coin_21".to_string(),
        "coin_22".to_string(),
        "coin_23".to_string(),
        "coin_24".to_string(),
        "coin_25".to_string(),
        "coin_26".to_string(),
        "coin_27".to_string(),
        "coin_28".to_string(),
        "coin_29".to_string(),
        "coin_30".to_string(),
        "coin_31".to_string(),
        "coin_32".to_string(),
    ];

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec![],
        allowed_coins: allowed_coins.clone(),
        red_bank: RedBankBase::new("red_bank_contract".to_string()),
        oracle: OracleBase::new("oracle_contract".to_string()),
    };

    let contract_addr = app
        .instantiate_contract(code_id, owner, &msg, &[], "mock-contract", None)
        .unwrap();

    let coins_res: Vec<String> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedCoins {
                start_after: None,
                limit: Some(58u32),
            },
        )
        .unwrap();

    // Assert maximum is observed
    assert_eq!(coins_res.len(), 30);

    let coins_res: Vec<String> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedCoins {
                start_after: None,
                limit: Some(2u32),
            },
        )
        .unwrap();

    // Assert limit request is observed
    assert_eq!(coins_res.len(), 2);

    let coins_res_a: Vec<String> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedCoins {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    let coins_res_b: Vec<String> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedCoins {
                start_after: Some(coins_res_a.last().unwrap().clone()),
                limit: None,
            },
        )
        .unwrap();

    let coins_res_c: Vec<String> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.clone(),
            &QueryMsg::AllowedCoins {
                start_after: Some(coins_res_b.last().unwrap().clone()),
                limit: None,
            },
        )
        .unwrap();

    let coins_res_d: Vec<String> = app
        .wrap()
        .query_wasm_smart(
            contract_addr,
            &QueryMsg::AllowedCoins {
                start_after: Some(coins_res_c.last().unwrap().clone()),
                limit: None,
            },
        )
        .unwrap();

    // Assert default is observed
    assert_eq!(coins_res_a.len(), 10);
    assert_eq!(coins_res_b.len(), 10);
    assert_eq!(coins_res_c.len(), 10);

    assert_eq!(coins_res_d.len(), 2);

    let combined: Vec<String> = coins_res_a
        .iter()
        .cloned()
        .chain(coins_res_b.iter().cloned())
        .chain(coins_res_c.iter().cloned())
        .chain(coins_res_d.iter().cloned())
        .collect();

    assert_eq!(combined.len(), allowed_coins.len());
    assert!(allowed_coins.iter().all(|item| combined.contains(item)));
}
