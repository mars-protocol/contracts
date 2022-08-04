use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use credit_manager::borrow::DEFAULT_DEBT_UNITS_PER_COIN_BORROWED;
use cw_multi_test::{App, Executor};

use rover::msg::execute::Action;
use rover::msg::query::CoinShares;
use rover::msg::{ExecuteMsg, QueryMsg};

use crate::helpers::{
    fund_red_bank, get_token_id, mock_create_credit_account, query_config, setup_credit_manager,
    CoinPriceLTV,
};

pub mod helpers;

#[test]
fn test_pagination_on_all_total_debt_shares_query_works() {
    let user_a = Addr::unchecked("user_a");
    let user_b = Addr::unchecked("user_b");
    let user_c = Addr::unchecked("user_c");

    let user_a_coins = vec![
        Coin::new(10u128, "coin_1"),
        Coin::new(10u128, "coin_2"),
        Coin::new(10u128, "coin_3"),
        Coin::new(10u128, "coin_4"),
        Coin::new(10u128, "coin_5"),
        Coin::new(10u128, "coin_6"),
        Coin::new(10u128, "coin_7"),
        Coin::new(10u128, "coin_8"),
        Coin::new(10u128, "coin_9"),
        Coin::new(10u128, "coin_10"),
        Coin::new(10u128, "coin_11"),
        Coin::new(10u128, "coin_12"),
        Coin::new(10u128, "coin_13"),
        Coin::new(10u128, "coin_14"),
    ];

    let user_b_coins = vec![
        Coin::new(10u128, "coin_15"),
        Coin::new(10u128, "coin_16"),
        Coin::new(10u128, "coin_17"),
        Coin::new(10u128, "coin_18"),
        Coin::new(10u128, "coin_19"),
        Coin::new(10u128, "coin_20"),
        Coin::new(10u128, "coin_21"),
        Coin::new(10u128, "coin_22"),
        Coin::new(10u128, "coin_23"),
        Coin::new(10u128, "coin_24"),
    ];

    let user_c_coins = vec![
        Coin::new(10u128, "coin_25"),
        Coin::new(10u128, "coin_26"),
        Coin::new(10u128, "coin_27"),
        Coin::new(10u128, "coin_28"),
        Coin::new(10u128, "coin_29"),
        Coin::new(10u128, "coin_30"),
        Coin::new(10u128, "coin_31"),
        Coin::new(10u128, "coin_32"),
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
            CoinPriceLTV {
                denom: "coin_1".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_2".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_3".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_4".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_5".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_6".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_7".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_8".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_9".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_10".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_11".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_12".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_13".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_14".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_15".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_16".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_17".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_18".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_19".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_20".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_21".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_22".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_23".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_24".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_25".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_26".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_27".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_28".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_29".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_30".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_31".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
            CoinPriceLTV {
                denom: "coin_32".to_string(),
                max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
                price: Decimal::from_atomics(10u128, 0).unwrap(),
            },
        ],
        vec![],
    );

    let config = query_config(&mut app, &mock.credit_manager.clone());

    fund_red_bank(
        &mut app,
        config.red_bank.clone(),
        vec![
            Coin::new(1000u128, "coin_1"),
            Coin::new(1000u128, "coin_2"),
            Coin::new(1000u128, "coin_3"),
            Coin::new(1000u128, "coin_4"),
            Coin::new(1000u128, "coin_5"),
            Coin::new(1000u128, "coin_6"),
            Coin::new(1000u128, "coin_7"),
            Coin::new(1000u128, "coin_8"),
            Coin::new(1000u128, "coin_9"),
            Coin::new(1000u128, "coin_10"),
            Coin::new(1000u128, "coin_11"),
            Coin::new(1000u128, "coin_12"),
            Coin::new(1000u128, "coin_13"),
            Coin::new(1000u128, "coin_14"),
            Coin::new(1000u128, "coin_15"),
            Coin::new(1000u128, "coin_16"),
            Coin::new(1000u128, "coin_17"),
            Coin::new(1000u128, "coin_18"),
            Coin::new(1000u128, "coin_19"),
            Coin::new(1000u128, "coin_20"),
            Coin::new(1000u128, "coin_21"),
            Coin::new(1000u128, "coin_22"),
            Coin::new(1000u128, "coin_23"),
            Coin::new(1000u128, "coin_24"),
            Coin::new(1000u128, "coin_25"),
            Coin::new(1000u128, "coin_26"),
            Coin::new(1000u128, "coin_27"),
            Coin::new(1000u128, "coin_28"),
            Coin::new(1000u128, "coin_29"),
            Coin::new(1000u128, "coin_30"),
            Coin::new(1000u128, "coin_31"),
            Coin::new(1000u128, "coin_32"),
        ],
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
                .flat_map(|coin| {
                    vec![
                        Action::Deposit(coin.clone()),
                        Action::Borrow(Coin {
                            denom: coin.denom.clone(),
                            amount: Uint128::from(1u128),
                        }),
                    ]
                })
                .collect::<Vec<Action>>(),
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
                .flat_map(|coin| {
                    vec![
                        Action::Deposit(coin.clone()),
                        Action::Borrow(Coin {
                            denom: coin.denom.clone(),
                            amount: Uint128::from(1u128),
                        }),
                    ]
                })
                .collect::<Vec<Action>>(),
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
                .flat_map(|coin| {
                    vec![
                        Action::Deposit(coin.clone()),
                        Action::Borrow(Coin {
                            denom: coin.denom.clone(),
                            amount: Uint128::from(1u128),
                        }),
                    ]
                })
                .collect::<Vec<Action>>(),
        },
        &user_c_coins,
    )
    .unwrap();

    let all_total_debt_shares_res: Vec<CoinShares> = app
        .wrap()
        .query_wasm_smart(
            mock.credit_manager.clone(),
            &QueryMsg::AllTotalDebtShares {
                start_after: None,
                limit: Some(58 as u32),
            },
        )
        .unwrap();

    // Assert maximum is observed
    assert_eq!(all_total_debt_shares_res.len(), 30);

    let all_total_debt_shares_res: Vec<CoinShares> = app
        .wrap()
        .query_wasm_smart(
            mock.credit_manager.clone(),
            &QueryMsg::AllTotalDebtShares {
                start_after: None,
                limit: Some(2 as u32),
            },
        )
        .unwrap();

    // Assert limit request is observed
    assert_eq!(all_total_debt_shares_res.len(), 2);

    let all_total_debt_shares_res_a: Vec<CoinShares> = app
        .wrap()
        .query_wasm_smart(
            mock.credit_manager.clone(),
            &QueryMsg::AllTotalDebtShares {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    let CoinShares { denom, .. } = all_total_debt_shares_res_a.last().unwrap().clone();
    let all_total_debt_shares_res_b: Vec<CoinShares> = app
        .wrap()
        .query_wasm_smart(
            mock.credit_manager.clone(),
            &QueryMsg::AllTotalDebtShares {
                start_after: Some(denom),
                limit: None,
            },
        )
        .unwrap();

    let CoinShares { denom, .. } = all_total_debt_shares_res_b.last().unwrap().clone();
    let all_total_debt_shares_res_c: Vec<CoinShares> = app
        .wrap()
        .query_wasm_smart(
            mock.credit_manager.clone(),
            &QueryMsg::AllTotalDebtShares {
                start_after: Some(denom),
                limit: None,
            },
        )
        .unwrap();

    let CoinShares { denom, .. } = all_total_debt_shares_res_c.last().unwrap().clone();
    let all_total_debt_shares_res_d: Vec<CoinShares> = app
        .wrap()
        .query_wasm_smart(
            mock.credit_manager.clone(),
            &QueryMsg::AllTotalDebtShares {
                start_after: Some(denom),
                limit: None,
            },
        )
        .unwrap();

    // Assert default is observed
    assert_eq!(all_total_debt_shares_res_a.len(), 10);
    assert_eq!(all_total_debt_shares_res_b.len(), 10);
    assert_eq!(all_total_debt_shares_res_c.len(), 10);

    assert_eq!(all_total_debt_shares_res_d.len(), 2);

    let combined_res: Vec<CoinShares> = all_total_debt_shares_res_a
        .iter()
        .cloned()
        .chain(all_total_debt_shares_res_b.iter().cloned())
        .chain(all_total_debt_shares_res_c.iter().cloned())
        .chain(all_total_debt_shares_res_d.iter().cloned())
        .collect();

    let user_a_response_items = user_a_coins
        .iter()
        .map(|coin| CoinShares {
            denom: coin.denom.clone(),
            shares: Uint128::from(DEFAULT_DEBT_UNITS_PER_COIN_BORROWED),
        })
        .collect::<Vec<CoinShares>>();

    let user_b_response_items = user_b_coins
        .iter()
        .map(|coin| CoinShares {
            denom: coin.denom.clone(),
            shares: Uint128::from(DEFAULT_DEBT_UNITS_PER_COIN_BORROWED),
        })
        .collect::<Vec<CoinShares>>();

    let user_c_response_items = user_c_coins
        .iter()
        .map(|coin| CoinShares {
            denom: coin.denom.clone(),
            shares: Uint128::from(DEFAULT_DEBT_UNITS_PER_COIN_BORROWED),
        })
        .collect::<Vec<CoinShares>>();

    let combined_starting_vals: Vec<CoinShares> = user_a_response_items
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
