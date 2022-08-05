use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_multi_test::{App, Executor};

use rover::msg::execute::Action;
use rover::msg::query::AssetResponseItem;
use rover::msg::{ExecuteMsg, QueryMsg};

use crate::helpers::{get_token_id, mock_create_credit_account, setup_credit_manager, CoinInfo};

pub mod helpers;

#[test]
fn test_pagination_on_all_coin_assets_query_works() {
    let user_a = Addr::unchecked("user_a");
    let user_b = Addr::unchecked("user_b");
    let user_c = Addr::unchecked("user_c");

    let user_a_coins = vec![
        Coin::new(1u128, "coin_1"),
        Coin::new(1u128, "coin_2"),
        Coin::new(1u128, "coin_3"),
        Coin::new(1u128, "coin_4"),
        Coin::new(1u128, "coin_5"),
        Coin::new(1u128, "coin_6"),
        Coin::new(1u128, "coin_7"),
        Coin::new(1u128, "coin_8"),
        Coin::new(1u128, "coin_9"),
        Coin::new(1u128, "coin_10"),
        Coin::new(1u128, "coin_11"),
        Coin::new(1u128, "coin_12"),
        Coin::new(1u128, "coin_13"),
        Coin::new(1u128, "coin_14"),
    ];

    let user_b_coins = vec![
        Coin::new(1u128, "coin_1"),
        Coin::new(1u128, "coin_2"),
        Coin::new(1u128, "coin_3"),
        Coin::new(1u128, "coin_4"),
        Coin::new(1u128, "coin_5"),
        Coin::new(1u128, "coin_6"),
        Coin::new(1u128, "coin_7"),
        Coin::new(1u128, "coin_8"),
        Coin::new(1u128, "coin_9"),
        Coin::new(1u128, "coin_10"),
    ];

    let user_c_coins = vec![
        Coin::new(1u128, "coin_1"),
        Coin::new(1u128, "coin_2"),
        Coin::new(1u128, "coin_3"),
        Coin::new(1u128, "coin_4"),
        Coin::new(1u128, "coin_5"),
        Coin::new(1u128, "coin_6"),
        Coin::new(1u128, "coin_7"),
        Coin::new(1u128, "coin_8"),
    ];

    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &user_a, user_a_coins.clone())
            .unwrap();
        router
            .bank
            .init_balance(storage, &user_b, user_b_coins.clone())
            .unwrap();
        router
            .bank
            .init_balance(storage, &user_c, user_c_coins.clone())
            .unwrap();
    });

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![
            CoinInfo {
                denom: "coin_1".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinInfo {
                denom: "coin_2".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinInfo {
                denom: "coin_3".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinInfo {
                denom: "coin_4".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinInfo {
                denom: "coin_5".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinInfo {
                denom: "coin_6".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinInfo {
                denom: "coin_7".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinInfo {
                denom: "coin_8".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinInfo {
                denom: "coin_9".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinInfo {
                denom: "coin_10".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinInfo {
                denom: "coin_11".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinInfo {
                denom: "coin_12".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinInfo {
                denom: "coin_13".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinInfo {
                denom: "coin_14".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
        ],
        vec![],
    );

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user_a).unwrap();
    let token_id_a = get_token_id(res);
    app.execute_contract(
        user_a.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id_a.clone(),
            actions: user_a_coins
                .iter()
                .map(|coin| Action::Deposit(coin.clone()))
                .collect(),
        },
        &user_a_coins,
    )
    .unwrap();

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user_b).unwrap();
    let token_id_b = get_token_id(res);
    app.execute_contract(
        user_b.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id_b.clone(),
            actions: user_b_coins
                .iter()
                .map(|coin| Action::Deposit(coin.clone()))
                .collect(),
        },
        &user_b_coins,
    )
    .unwrap();

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user_c).unwrap();
    let token_id_c = get_token_id(res);
    app.execute_contract(
        user_c.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id_c.clone(),
            actions: user_c_coins
                .iter()
                .map(|coin| Action::Deposit(coin.clone()))
                .collect(),
        },
        &user_c_coins,
    )
    .unwrap();

    let all_assets_res: Vec<AssetResponseItem> = app
        .wrap()
        .query_wasm_smart(
            mock.credit_manager.clone(),
            &QueryMsg::AllAssets {
                start_after: None,
                limit: Some(58 as u32),
            },
        )
        .unwrap();

    // Assert maximum is observed
    assert_eq!(all_assets_res.len(), 30);

    let all_assets_res: Vec<AssetResponseItem> = app
        .wrap()
        .query_wasm_smart(
            mock.credit_manager.clone(),
            &QueryMsg::AllAssets {
                start_after: None,
                limit: Some(2 as u32),
            },
        )
        .unwrap();

    // Assert limit request is observed
    assert_eq!(all_assets_res.len(), 2);

    let all_assets_res_a: Vec<AssetResponseItem> = app
        .wrap()
        .query_wasm_smart(
            mock.credit_manager.clone(),
            &QueryMsg::AllAssets {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    let AssetResponseItem {
        token_id, denom, ..
    } = all_assets_res_a.last().unwrap().clone();
    let all_assets_res_b: Vec<AssetResponseItem> = app
        .wrap()
        .query_wasm_smart(
            mock.credit_manager.clone(),
            &QueryMsg::AllAssets {
                start_after: Some((token_id, denom)),
                limit: None,
            },
        )
        .unwrap();

    let AssetResponseItem {
        token_id, denom, ..
    } = all_assets_res_b.last().unwrap().clone();
    let all_assets_res_c: Vec<AssetResponseItem> = app
        .wrap()
        .query_wasm_smart(
            mock.credit_manager.clone(),
            &QueryMsg::AllAssets {
                start_after: Some((token_id, denom)),
                limit: None,
            },
        )
        .unwrap();

    let AssetResponseItem {
        token_id, denom, ..
    } = all_assets_res_c.last().unwrap().clone();
    let all_assets_res_d: Vec<AssetResponseItem> = app
        .wrap()
        .query_wasm_smart(
            mock.credit_manager.clone(),
            &QueryMsg::AllAssets {
                start_after: Some((token_id, denom)),
                limit: None,
            },
        )
        .unwrap();

    // Assert default is observed
    assert_eq!(all_assets_res_a.len(), 10);
    assert_eq!(all_assets_res_b.len(), 10);
    assert_eq!(all_assets_res_c.len(), 10);

    assert_eq!(all_assets_res_d.len(), 2);

    let combined_res: Vec<AssetResponseItem> = all_assets_res_a
        .iter()
        .cloned()
        .chain(all_assets_res_b.iter().cloned())
        .chain(all_assets_res_c.iter().cloned())
        .chain(all_assets_res_d.iter().cloned())
        .collect();

    let user_a_response_items = user_a_coins
        .iter()
        .map(|coin| AssetResponseItem {
            token_id: token_id_a.clone(),
            denom: coin.denom.clone(),
            amount: Uint128::from(1u128),
        })
        .collect::<Vec<AssetResponseItem>>();

    let user_b_response_items = user_b_coins
        .iter()
        .map(|coin| AssetResponseItem {
            token_id: token_id_b.clone(),
            denom: coin.denom.clone(),
            amount: Uint128::from(1u128),
        })
        .collect::<Vec<AssetResponseItem>>();

    let user_c_response_items = user_c_coins
        .iter()
        .map(|coin| AssetResponseItem {
            token_id: token_id_c.clone(),
            denom: coin.denom.clone(),
            amount: Uint128::from(1u128),
        })
        .collect::<Vec<AssetResponseItem>>();

    let combined_starting_vals: Vec<AssetResponseItem> = user_a_response_items
        .iter()
        .cloned()
        .chain(user_b_response_items)
        .chain(user_c_response_items)
        .collect();

    assert_eq!(combined_res.len(), combined_starting_vals.len());
    assert!(combined_starting_vals
        .iter()
        .all(|item| combined_res.contains(item)));
}
