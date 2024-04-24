use cosmwasm_std::{testing::mock_env, Addr, Decimal, Uint128};
use cw_paginate::{Metadata, PaginationResponse};
use mars_interest_rate::{
    get_scaled_debt_amount, get_underlying_debt_amount, get_underlying_liquidity_amount,
    SCALING_FACTOR,
};
use mars_red_bank::{
    query::{query_user_collaterals, query_user_collaterals_v2, query_user_debt, query_user_debts},
    state::DEBTS,
};
use mars_types::red_bank::{
    Debt, Market, MarketV2Response, QueryMsg, UserCollateralResponse, UserDebtResponse,
};

use super::helpers::{set_collateral, th_init_market, th_query, th_setup};

#[test]
fn query_collateral() {
    let mut deps = th_setup(&[]);

    let user_addr = Addr::unchecked("user");

    // Setup first market
    let market_1 = th_init_market(deps.as_mut(), "uosmo", &Default::default());

    // Setup second market
    let market_2 = th_init_market(deps.as_mut(), "uusd", &Default::default());

    let amount_1 = Uint128::new(12345);
    let amount_2 = Uint128::new(54321);

    let env = mock_env();

    // Create and enable a collateral position for the 2nd asset
    set_collateral(deps.as_mut(), &user_addr, &market_2.denom, amount_2 * SCALING_FACTOR, true);

    // Assert markets correctly return collateral status
    let collaterals =
        query_user_collaterals(deps.as_ref(), &env.block, user_addr.clone(), None, None, None)
            .unwrap();
    assert_eq!(
        collaterals,
        vec![UserCollateralResponse {
            denom: market_2.denom.clone(),
            amount_scaled: amount_2 * SCALING_FACTOR,
            amount: amount_2,
            enabled: true,
        }]
    );

    // Create a collateral position for the 1st asset, but not enabled
    set_collateral(deps.as_mut(), &user_addr, &market_1.denom, amount_1 * SCALING_FACTOR, false);

    // Assert markets correctly return collateral status
    let collaterals =
        query_user_collaterals(deps.as_ref(), &env.block, user_addr, None, None, None).unwrap();
    assert_eq!(
        collaterals,
        vec![
            UserCollateralResponse {
                denom: market_1.denom,
                amount_scaled: amount_1 * SCALING_FACTOR,
                amount: amount_1,
                enabled: false,
            },
            UserCollateralResponse {
                denom: market_2.denom,
                amount_scaled: amount_2 * SCALING_FACTOR,
                amount: amount_2,
                enabled: true,
            },
        ]
    );
}

#[test]
fn paginate_user_collaterals_v2() {
    let mut deps = th_setup(&[]);
    let env = mock_env();

    let user_addr = Addr::unchecked("user");

    let market_1 = th_init_market(deps.as_mut(), "uosmo", &Default::default());
    let market_2 = th_init_market(deps.as_mut(), "uatom", &Default::default());
    let market_3 = th_init_market(deps.as_mut(), "untrn", &Default::default());
    let market_4 = th_init_market(deps.as_mut(), "ujuno", &Default::default());
    let market_5 = th_init_market(deps.as_mut(), "uusdc", &Default::default());
    let market_6 = th_init_market(deps.as_mut(), "ujake", &Default::default());

    set_collateral(deps.as_mut(), &user_addr, &market_1.denom, Uint128::one(), true);
    set_collateral(deps.as_mut(), &user_addr, &market_2.denom, Uint128::one(), true);
    set_collateral(deps.as_mut(), &user_addr, &market_3.denom, Uint128::one(), true);
    set_collateral(deps.as_mut(), &user_addr, &market_4.denom, Uint128::one(), true);
    set_collateral(deps.as_mut(), &user_addr, &market_5.denom, Uint128::one(), true);
    set_collateral(deps.as_mut(), &user_addr, &market_6.denom, Uint128::one(), false);

    // Check pagination with default params
    let collaterals =
        query_user_collaterals_v2(deps.as_ref(), &env.block, user_addr.clone(), None, None, None)
            .unwrap();
    assert_eq!(
        to_denoms(&collaterals.data),
        vec!["uatom", "ujake", "ujuno", "untrn", "uosmo", "uusdc"]
    );
    assert!(!collaterals.metadata.has_more);

    // Paginate all collaterals
    let collaterals = query_user_collaterals_v2(
        deps.as_ref(),
        &env.block,
        user_addr.clone(),
        None,
        Some("uatom".to_string()),
        Some(2),
    )
    .unwrap();
    assert_eq!(to_denoms(&collaterals.data), vec!["ujake", "ujuno"]);
    assert!(collaterals.metadata.has_more);

    let collaterals = query_user_collaterals_v2(
        deps.as_ref(),
        &env.block,
        user_addr.clone(),
        None,
        Some("ujuno".to_string()),
        Some(2),
    )
    .unwrap();
    assert_eq!(to_denoms(&collaterals.data), vec!["untrn", "uosmo"]);
    assert!(collaterals.metadata.has_more);

    let collaterals = query_user_collaterals_v2(
        deps.as_ref(),
        &env.block,
        user_addr,
        None,
        Some("uosmo".to_string()),
        Some(2),
    )
    .unwrap();
    assert_eq!(to_denoms(&collaterals.data), vec!["uusdc"]);
    assert!(!collaterals.metadata.has_more);
}

fn to_denoms(res: &[UserCollateralResponse]) -> Vec<&str> {
    res.iter().map(|item| item.denom.as_str()).collect()
}

#[test]
fn test_query_user_debt() {
    let mut deps = th_setup(&[]);

    let user_addr = Addr::unchecked("user");

    // Setup markets
    let market_1_initial = th_init_market(
        deps.as_mut(),
        "coin_1",
        &Market {
            borrow_index: Decimal::one(),
            borrow_rate: Decimal::one(),
            ..Default::default()
        },
    );
    let _market_2_initial = th_init_market(
        deps.as_mut(),
        "coin_2",
        &Market {
            borrow_index: Decimal::one(),
            borrow_rate: Decimal::one(),
            ..Default::default()
        },
    );
    let market_3_initial = th_init_market(
        deps.as_mut(),
        "coin_3",
        &Market {
            borrow_index: Decimal::one(),
            borrow_rate: Decimal::one(),
            ..Default::default()
        },
    );

    let env = mock_env();

    // Save debt for market 1
    let debt_amount_1 = Uint128::new(1234000u128);
    let debt_amount_scaled_1 =
        get_scaled_debt_amount(debt_amount_1, &market_1_initial, env.block.time.seconds()).unwrap();
    let debt_amount_at_query_1 = get_underlying_debt_amount(
        debt_amount_scaled_1,
        &market_1_initial,
        env.block.time.seconds(),
    )
    .unwrap();
    let debt_1 = Debt {
        amount_scaled: debt_amount_scaled_1,
        uncollateralized: false,
    };
    DEBTS.save(deps.as_mut().storage, (&user_addr, "coin_1"), &debt_1).unwrap();

    // Save debt for market 3
    let debt_amount_3 = Uint128::new(2221u128);
    let debt_amount_scaled_3 =
        get_scaled_debt_amount(debt_amount_3, &market_3_initial, env.block.time.seconds()).unwrap();
    let debt_amount_at_query_3 = get_underlying_debt_amount(
        debt_amount_scaled_3,
        &market_3_initial,
        env.block.time.seconds(),
    )
    .unwrap();
    let debt_3 = Debt {
        amount_scaled: debt_amount_scaled_3,
        uncollateralized: false,
    };
    DEBTS.save(deps.as_mut().storage, (&user_addr, "coin_3"), &debt_3).unwrap();

    let debts = query_user_debts(deps.as_ref(), &env.block, user_addr, None, None).unwrap();
    assert_eq!(debts.len(), 2);
    assert_eq!(
        debts[0],
        UserDebtResponse {
            denom: "coin_1".to_string(),
            amount_scaled: debt_amount_scaled_1,
            amount: debt_amount_at_query_1,
            uncollateralized: false,
        }
    );
    assert_eq!(
        debts[1],
        UserDebtResponse {
            denom: "coin_3".to_string(),
            amount_scaled: debt_amount_scaled_3,
            amount: debt_amount_at_query_3,
            uncollateralized: false,
        }
    );
}

#[test]
fn query_user_asset_debt() {
    let mut deps = th_setup(&[]);

    let user_addr = Addr::unchecked("user");

    // Setup markets
    let market_1_initial = th_init_market(
        deps.as_mut(),
        "coin_1",
        &Market {
            borrow_index: Decimal::one(),
            borrow_rate: Decimal::one(),
            ..Default::default()
        },
    );
    let _market_2_initial = th_init_market(
        deps.as_mut(),
        "coin_2",
        &Market {
            borrow_index: Decimal::one(),
            borrow_rate: Decimal::one(),
            ..Default::default()
        },
    );

    let env = mock_env();

    // Save debt for market 1
    let debt_amount_1 = Uint128::new(1234567u128);
    let debt_amount_scaled_1 =
        get_scaled_debt_amount(debt_amount_1, &market_1_initial, env.block.time.seconds()).unwrap();
    let debt_amount_at_query_1 = get_underlying_debt_amount(
        debt_amount_scaled_1,
        &market_1_initial,
        env.block.time.seconds(),
    )
    .unwrap();
    let debt_1 = Debt {
        amount_scaled: debt_amount_scaled_1,
        uncollateralized: false,
    };
    DEBTS.save(deps.as_mut().storage, (&user_addr, "coin_1"), &debt_1).unwrap();

    // Check asset with existing debt
    {
        let res =
            query_user_debt(deps.as_ref(), &env.block, user_addr.clone(), "coin_1".to_string())
                .unwrap();
        assert_eq!(
            res,
            UserDebtResponse {
                denom: "coin_1".to_string(),
                amount_scaled: debt_amount_scaled_1,
                amount: debt_amount_at_query_1,
                uncollateralized: false,
            }
        );
    }

    // Check asset with no debt
    {
        let res =
            query_user_debt(deps.as_ref(), &env.block, user_addr, "coin_2".to_string()).unwrap();
        assert_eq!(
            res,
            UserDebtResponse {
                denom: "coin_2".to_string(),
                amount_scaled: Uint128::zero(),
                amount: Uint128::zero(),
                uncollateralized: false,
            }
        );
    }
}

#[test]
fn query_single_market_v2() {
    let mut deps = th_setup(&[]);
    let env = mock_env();
    let market = th_init_market(
        deps.as_mut(),
        "uosmo",
        &Market {
            borrow_index: Decimal::from_atomics(11u128, 2).unwrap(),
            borrow_rate: Decimal::from_atomics(123u128, 2).unwrap(),
            debt_total_scaled: Uint128::new(500000u128),
            liquidity_rate: Decimal::from_atomics(22u128, 2).unwrap(),
            liquidity_index: Decimal::from_atomics(456u128, 2).unwrap(),
            collateral_total_scaled: Uint128::new(1000000u128),
            ..Default::default()
        },
    );

    let market_response: MarketV2Response = th_query(
        deps.as_ref(),
        QueryMsg::MarketV2 {
            denom: market.denom.clone(),
        },
    );

    let debt_total_amount =
        get_underlying_debt_amount(market.debt_total_scaled, &market, env.block.time.seconds())
            .unwrap();
    let collateral_total_amount = get_underlying_liquidity_amount(
        market.collateral_total_scaled,
        &market,
        env.block.time.seconds(),
    )
    .unwrap();
    let utilization_rate = Decimal::from_ratio(debt_total_amount, collateral_total_amount);

    assert_eq!(
        market_response,
        MarketV2Response {
            debt_total_amount,
            collateral_total_amount,
            utilization_rate,
            market,
        }
    );
}

#[test]
fn query_all_markets_v2() {
    let mut deps = th_setup(&[]);
    let env = mock_env();
    let market_1 = th_init_market(
        deps.as_mut(),
        "uosmo",
        &Market {
            borrow_index: Decimal::from_atomics(11u128, 2).unwrap(),
            borrow_rate: Decimal::from_atomics(102u128, 2).unwrap(),
            debt_total_scaled: Uint128::new(500000u128),
            liquidity_rate: Decimal::from_atomics(66u128, 2).unwrap(),
            liquidity_index: Decimal::from_atomics(101u128, 2).unwrap(),
            collateral_total_scaled: Uint128::new(1000000u128),
            ..Default::default()
        },
    );

    let market_2 = th_init_market(
        deps.as_mut(),
        "atom",
        &Market {
            borrow_index: Decimal::from_atomics(22u128, 2).unwrap(),
            borrow_rate: Decimal::from_atomics(103u128, 2).unwrap(),
            debt_total_scaled: Uint128::new(1000000u128),
            liquidity_rate: Decimal::from_atomics(77u128, 2).unwrap(),
            liquidity_index: Decimal::from_atomics(104u128, 2).unwrap(),
            collateral_total_scaled: Uint128::new(1500000u128),
            ..Default::default()
        },
    );

    let markets_response: PaginationResponse<MarketV2Response> = th_query(
        deps.as_ref(),
        QueryMsg::MarketsV2 {
            start_after: None,
            limit: None,
        },
    );

    let debt_total_amount_1 =
        get_underlying_debt_amount(market_1.debt_total_scaled, &market_1, env.block.time.seconds())
            .unwrap();
    let collateral_total_amount_1 = get_underlying_liquidity_amount(
        market_1.collateral_total_scaled,
        &market_1,
        env.block.time.seconds(),
    )
    .unwrap();
    let utilization_rate_1 = Decimal::from_ratio(debt_total_amount_1, collateral_total_amount_1);
    let debt_total_amount_2 =
        get_underlying_debt_amount(market_2.debt_total_scaled, &market_2, env.block.time.seconds())
            .unwrap();
    let collateral_total_amount_2 = get_underlying_liquidity_amount(
        market_2.collateral_total_scaled,
        &market_2,
        env.block.time.seconds(),
    )
    .unwrap();
    let utilization_rate_2 = Decimal::from_ratio(debt_total_amount_2, collateral_total_amount_2);

    let data = vec![
        MarketV2Response {
            debt_total_amount: debt_total_amount_2,
            collateral_total_amount: collateral_total_amount_2,
            utilization_rate: utilization_rate_2,
            market: market_2,
        },
        MarketV2Response {
            debt_total_amount: debt_total_amount_1,
            collateral_total_amount: collateral_total_amount_1,
            utilization_rate: utilization_rate_1,
            market: market_1,
        },
    ];

    assert_eq!(
        markets_response,
        PaginationResponse {
            data,
            metadata: Metadata {
                has_more: false,
            },
        }
    );
}
