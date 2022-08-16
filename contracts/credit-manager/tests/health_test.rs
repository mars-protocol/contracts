use std::ops::{Add, Div, Mul};

use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_multi_test::{App, BasicApp, Executor};

use credit_manager::borrow::DEFAULT_DEBT_UNITS_PER_COIN_BORROWED;
use mock_oracle::msg::{CoinPrice, ExecuteMsg as OracleExecuteMsg};
use rover::error::ContractError;
use rover::msg::execute::Action::{Borrow, Deposit};
use rover::msg::query::{CoinShares, DebtSharesValue};
use rover::msg::{ExecuteMsg, QueryMsg};

use crate::helpers::{
    assert_err, fund_red_bank, get_token_id, mock_app, mock_create_credit_account, query_config,
    query_health, query_position, query_red_bank_debt, setup_credit_manager, CoinInfo, MockEnv,
};

pub mod helpers;

/// Action: User deposits 300 osmo (.25 price)
/// Health: assets_value: 75
///         debt value 0
///         liquidatable: false
///         above_max_ltv: false
#[test]
fn test_only_assets_with_no_debts() {
    let user = Addr::unchecked("user");
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &user, vec![Coin::new(300u128, "uosmo")])
            .unwrap();
    });

    let coin_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(25u128, 2).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
    };

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![coin_info.clone()],
        vec![],
    );

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let deposit_amount = Uint128::from(300u128);

    app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Deposit(coin_info.to_coin(deposit_amount))],
        },
        &[Coin::new(deposit_amount.into(), "uosmo")],
    )
    .unwrap();

    let position = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(position.coins.len(), 1);
    assert_eq!(position.debt_shares.len(), 0);

    let health = query_health(&app, &mock.credit_manager, &token_id);
    let assets_value = coin_info.price * Decimal::from_atomics(deposit_amount, 0).unwrap();
    assert_eq!(health.total_assets_value, assets_value);
    assert_eq!(health.total_debts_value, Decimal::zero());
    assert_eq!(health.lqdt_health_factor, None);
    assert_eq!(health.max_ltv_health_factor, None);
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);
}

/// Step 1: User deposits 12 luna (100 price) and borrows 2 luna
/// Health: assets_value: 1400
///         debt value 200
///         liquidatable: false
///         above_max_ltv: false
/// Step 2: luna price goes to zero
/// Health: assets_value: 0
///         debt value 0 (still debt shares outstanding)
///         liquidatable: false
///         above_max_ltv: false
#[test]
fn test_terra_ragnarok() {
    let user = Addr::unchecked("user");
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &user, vec![Coin::new(300u128, "uluna")])
            .unwrap();
    });

    let coin_info = CoinInfo {
        denom: "uluna".to_string(),
        price: Decimal::from_atomics(100u128, 0).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
    };

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![coin_info.clone()],
        vec![],
    );

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let config = query_config(&app, &mock.credit_manager.clone());

    fund_red_bank(
        &mut app,
        config.red_bank,
        vec![Coin::new(1000u128, "uluna")],
    );

    let deposit_amount = Uint128::from(12u128);
    let borrow_amount = Uint128::from(2u128);
    app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![
                Deposit(coin_info.to_coin(deposit_amount)),
                Borrow(coin_info.to_coin(borrow_amount)),
            ],
        },
        &[Coin::new(deposit_amount.into(), "uluna")],
    )
    .unwrap();

    let position = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(position.coins.len(), 1);
    assert_eq!(position.debt_shares.len(), 1);

    let health = query_health(&app, &mock.credit_manager, &token_id);
    let assets_value =
        coin_info.price * Decimal::from_atomics(deposit_amount + borrow_amount, 0).unwrap();
    assert_eq!(health.total_assets_value, assets_value);
    // Note: Simulated yield from mock_red_bank makes debt position more expensive
    let debts_value = coin_info.price
        * Decimal::from_atomics(borrow_amount.add(Uint128::from(1u128)), 0).unwrap();
    assert_eq!(health.total_debts_value, debts_value);
    assert_eq!(
        health.lqdt_health_factor,
        Some(assets_value * coin_info.liquidation_threshold / debts_value)
    );
    assert_eq!(
        health.max_ltv_health_factor,
        Some(assets_value * coin_info.max_ltv / debts_value)
    );
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);

    price_change(
        &mut app,
        &mock,
        CoinPrice {
            denom: coin_info.denom,
            price: Decimal::zero(),
        },
    );

    let position = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(position.coins.len(), 1);
    assert_eq!(position.debt_shares.len(), 1);

    let health = query_health(&app, &mock.credit_manager, &token_id);
    assert_eq!(health.total_assets_value, Decimal::zero());
    assert_eq!(health.total_debts_value, Decimal::zero());
    assert_eq!(health.lqdt_health_factor, None);
    assert_eq!(health.max_ltv_health_factor, None);
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);
}

/// Action: User borrows 100 osmo (at price of 1). Zero deposits.
/// Health: assets_value: 100
///         debt value: 100
///         liquidatable: true
///         above_max_ltv: true
#[test]
fn test_debts_no_assets() {
    let user = Addr::unchecked("user");
    let mut app = mock_app();

    let coin_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::one(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
    };

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![coin_info.clone()],
        vec![],
    );
    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let config = query_config(&app, &mock.credit_manager.clone());

    fund_red_bank(
        &mut app,
        config.red_bank,
        vec![Coin::new(1000u128, coin_info.denom.clone())],
    );

    let borrowed_amount = Uint128::from(100u128);
    let res = app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Borrow(coin_info.to_coin(borrowed_amount))],
        },
        &[],
    );

    assert_err(res, ContractError::AboveMaxLTV);

    let position = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(position.token_id, token_id);
    assert_eq!(position.coins.len(), 0);
    assert_eq!(position.debt_shares.len(), 0);

    let health = query_health(&app, &mock.credit_manager, &token_id);
    assert_eq!(health.total_assets_value, Decimal::zero());
    assert_eq!(health.total_debts_value, Decimal::zero());
    assert_eq!(health.lqdt_health_factor, None);
    assert_eq!(health.max_ltv_health_factor, None);
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);
}

/// Step 1: User deposits 300 osmo and borrows 50 (at price of 2.3654)
/// Health: assets_value: 827.89
///         debt value: 121 (simulated interest incurred)
///         liquidatable: false
///         above_max_ltv: false
/// Step 2: User borrows 100
/// Health: assets_value: 1,064.43
///         debt value: 360 (simulated interest incurred)
///         liquidatable: false
///         above_max_ltv: false
/// Step 3: User borrows 100
///         AboveMaxLtv error thrown
#[test]
fn test_cannot_borrow_more_than_healthy() {
    let user = Addr::unchecked("user");
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &user, vec![Coin::new(300u128, "uosmo")])
            .unwrap();
    });

    let coin_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(23654u128, 4).unwrap(),
        max_ltv: Decimal::from_atomics(5u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap(),
    };

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![coin_info.clone()],
        vec![],
    );

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let config = query_config(&app, &mock.credit_manager.clone());

    fund_red_bank(
        &mut app,
        config.red_bank,
        vec![Coin::new(1000u128, coin_info.denom.clone())],
    );

    app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![
                Deposit(coin_info.to_coin(Uint128::from(300u128))),
                Borrow(coin_info.to_coin(Uint128::from(50u128))),
            ],
        },
        &[Coin::new(
            Uint128::from(300u128).into(),
            coin_info.denom.clone(),
        )],
    )
    .unwrap();

    let position = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(position.token_id, token_id);
    assert_eq!(position.coins.len(), 1);
    assert_eq!(position.debt_shares.len(), 1);

    let health = query_health(&app, &mock.credit_manager, &token_id);
    let assets_value = Decimal::from_atomics(82789u128, 2).unwrap();
    assert_eq!(health.total_assets_value, assets_value);
    let debts_value = Decimal::from_atomics(1206354u128, 4).unwrap();
    assert_eq!(health.total_debts_value, debts_value);
    assert_eq!(
        health.lqdt_health_factor,
        Some(assets_value * coin_info.liquidation_threshold / debts_value)
    );
    assert_eq!(
        health.max_ltv_health_factor,
        Some(assets_value * coin_info.max_ltv / debts_value)
    );
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);

    app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Borrow(coin_info.to_coin(Uint128::from(100u128)))],
        },
        &[],
    )
    .unwrap();

    let res = app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Borrow(coin_info.to_coin(Uint128::from(150u128)))],
        },
        &[],
    );

    assert_err(res, ContractError::AboveMaxLTV);

    // All valid on step 2 as well (meaning step 3 did not go through)
    let health = query_health(&app, &mock.credit_manager, &token_id);
    let assets_value = Decimal::from_atomics(106443u128, 2).unwrap();
    assert_eq!(health.total_assets_value, assets_value);
    let debts_value = Decimal::from_atomics(3595408u128, 4).unwrap();
    assert_eq!(health.total_debts_value, debts_value);
    assert_eq!(
        health.lqdt_health_factor,
        Some(assets_value * coin_info.liquidation_threshold / debts_value)
    );
    assert_eq!(
        health.max_ltv_health_factor,
        Some(assets_value * coin_info.max_ltv / debts_value)
    );
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);
}

/// Step 1: User deposits 300 osmo (2.3654) and borrows 50 atom (price 10.2)
/// Health: liquidatable: false
///         above_max_ltv: false
/// Step 2: Atom's price increases to 24
/// Health: liquidatable: false
///         above_max_ltv: true
/// Step 3: User borrows 2 atom
///         AboveMaxLtv error thrown
/// Step 4: Atom's price increases to 35
/// Health: liquidatable: true
///         above_max_ltv: true
#[test]
fn test_cannot_borrow_more_but_not_liquidatable() {
    let user = Addr::unchecked("user");
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &user, vec![Coin::new(300u128, "uosmo")])
            .unwrap();
    });

    let uosmo_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(23654u128, 4).unwrap(),
        max_ltv: Decimal::from_atomics(5u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap(),
    };

    let uatom_info = CoinInfo {
        denom: "uatom".to_string(),
        price: Decimal::from_atomics(102u128, 1).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(75u128, 2).unwrap(),
    };

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![uosmo_info.clone(), uatom_info.clone()],
        vec![],
    );

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let config = query_config(&app, &mock.credit_manager.clone());

    fund_red_bank(
        &mut app,
        config.red_bank,
        vec![Coin::new(1000u128, "uatom")],
    );

    app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![
                Deposit(uosmo_info.to_coin(Uint128::from(300u128))),
                Borrow(uatom_info.to_coin(Uint128::from(50u128))),
            ],
        },
        &[Coin::new(300, "uosmo")],
    )
    .unwrap();

    let health = query_health(&app, &mock.credit_manager, &token_id);
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);

    price_change(
        &mut app,
        &mock,
        CoinPrice {
            denom: uatom_info.denom.clone(),
            price: Decimal::from_atomics(24u128, 0).unwrap(),
        },
    );

    let health = query_health(&app, &mock.credit_manager, &token_id);
    assert!(!health.liquidatable);
    assert!(health.above_max_ltv);

    let res = app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Borrow(uatom_info.to_coin(Uint128::from(2u128)))],
        },
        &[],
    );

    assert_err(res, ContractError::AboveMaxLTV);

    price_change(
        &mut app,
        &mock,
        CoinPrice {
            denom: uatom_info.denom,
            price: Decimal::from_atomics(35u128, 0).unwrap(),
        },
    );

    let health = query_health(&app, &mock.credit_manager, &token_id);
    assert!(health.liquidatable);
    assert!(health.above_max_ltv);
}

/// Actions: User deposits 300 osmo (5265478965.412365487125 price)
///          and borrows 49 atom ( price)
/// Health: assets_value: 1569456334491.12991516325
///         debt value 350615100.25
///         liquidatable: false
///         above_max_ltv: false
#[test]
fn test_assets_and_ltv_lqdt_adjusted_value() {
    let user = Addr::unchecked("user");
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &user, vec![Coin::new(300u128, "uosmo")])
            .unwrap();
    });

    let uosmo_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(5265478965412365487125u128, 12).unwrap(),
        max_ltv: Decimal::from_atomics(6u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(7u128, 1).unwrap(),
    };

    let uatom_info = CoinInfo {
        denom: "uatom".to_string(),
        price: Decimal::from_atomics(7012302005u128, 3).unwrap(),
        max_ltv: Decimal::from_atomics(8u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(9u128, 1).unwrap(),
    };

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![uosmo_info.clone(), uatom_info.clone()],
        vec![],
    );

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let config = query_config(&app, &mock.credit_manager.clone());

    fund_red_bank(
        &mut app,
        config.red_bank,
        vec![Coin::new(1000u128, "uatom")],
    );

    let deposit_amount = Uint128::from(298u128);
    let borrowed_amount = Uint128::from(49u128);

    app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![
                Deposit(uosmo_info.to_coin(deposit_amount)),
                Borrow(uatom_info.to_coin(borrowed_amount)),
            ],
        },
        &[Coin::new(deposit_amount.into(), "uosmo")],
    )
    .unwrap();

    let position = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(position.token_id, token_id);
    assert_eq!(position.coins.len(), 2);
    assert_eq!(position.debt_shares.len(), 1);

    let health = query_health(&app, &mock.credit_manager, &token_id);
    let deposit_amount_dec = Decimal::from_atomics(deposit_amount, 0).unwrap();
    let borrowed_amount_dec = Decimal::from_atomics(borrowed_amount, 0).unwrap();
    assert_eq!(
        health.total_assets_value,
        uosmo_info.price * deposit_amount_dec + uatom_info.price * borrowed_amount_dec
    );
    assert_eq!(
        health.total_debts_value,
        uatom_info.price * (borrowed_amount_dec + Decimal::one()) // simulated interest
    );

    let lqdt_adjusted_assets_value =
        uosmo_info.price * deposit_amount_dec * uosmo_info.liquidation_threshold
            + uatom_info.price * borrowed_amount_dec * uatom_info.liquidation_threshold;
    assert_eq!(
        health.lqdt_health_factor,
        Some(
            lqdt_adjusted_assets_value
                .div(uatom_info.price.mul(borrowed_amount_dec + Decimal::one()))
        )
    );
    let ltv_adjusted_assets_value = uosmo_info.price * deposit_amount_dec * uosmo_info.max_ltv
        + uatom_info.price * borrowed_amount_dec * uatom_info.max_ltv;
    assert_eq!(
        health.max_ltv_health_factor,
        Some(
            ltv_adjusted_assets_value
                .div(uatom_info.price.mul(borrowed_amount_dec + Decimal::one()))
        )
    );
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);
}

/// User A: Borrows 30 osmo
///         Borrows 49 atom
///         Deposits 298 osmo
/// User B: Borrows 24 atom
///         Deposits 101 osmo
/// Test validates User A's debt value & health factors
#[test]
fn test_debt_value() {
    let user_a = Addr::unchecked("user_a");
    let user_b = Addr::unchecked("user_b");
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &user_a, vec![Coin::new(300u128, "uosmo")])
            .unwrap();
        router
            .bank
            .init_balance(storage, &user_b, vec![Coin::new(140u128, "uosmo")])
            .unwrap();
    });

    let uosmo_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(5265478965412365487125u128, 12).unwrap(),
        max_ltv: Decimal::from_atomics(3u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(5u128, 1).unwrap(),
    };

    let uatom_info = CoinInfo {
        denom: "uatom".to_string(),
        price: Decimal::from_atomics(7012302005u128, 3).unwrap(),
        max_ltv: Decimal::from_atomics(8u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(9u128, 1).unwrap(),
    };

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![uosmo_info.clone(), uatom_info.clone()],
        vec![],
    );

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user_a).unwrap();
    let token_id_a = get_token_id(res);

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user_b).unwrap();
    let token_id_b = get_token_id(res);

    let config = query_config(&app, &mock.credit_manager.clone());

    fund_red_bank(
        &mut app,
        config.red_bank.clone(),
        vec![Coin::new(1000u128, "uatom"), Coin::new(1000u128, "uosmo")],
    );

    let user_a_deposit_amount_osmo = Uint128::from(298u128);
    let user_a_borrowed_amount_atom = Uint128::from(49u128);
    let user_a_borrowed_amount_osmo = Uint128::from(30u128);

    app.execute_contract(
        user_a.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id_a.clone(),
            actions: vec![
                Borrow(uatom_info.to_coin(user_a_borrowed_amount_atom)),
                Borrow(uosmo_info.to_coin(user_a_borrowed_amount_osmo)),
                Deposit(uosmo_info.to_coin(user_a_deposit_amount_osmo)),
            ],
        },
        &[Coin::new(user_a_deposit_amount_osmo.into(), "uosmo")],
    )
    .unwrap();

    let interim_red_bank_debt = query_red_bank_debt(
        &app,
        &mock.credit_manager,
        &config.red_bank,
        &uatom_info.denom,
    );

    let user_b_deposit_amount = Uint128::from(101u128);
    let user_b_borrowed_amount_atom = Uint128::from(24u128);

    app.execute_contract(
        user_b.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id_b.clone(),
            actions: vec![
                Borrow(uatom_info.to_coin(user_b_borrowed_amount_atom)),
                Deposit(uosmo_info.to_coin(user_b_deposit_amount)),
            ],
        },
        &[Coin::new(user_b_deposit_amount.into(), "uosmo")],
    )
    .unwrap();

    let position_a = query_position(&app, &mock.credit_manager, &token_id_a);
    assert_eq!(position_a.token_id, token_id_a);
    assert_eq!(position_a.coins.len(), 2);
    assert_eq!(position_a.debt_shares.len(), 2);

    let health = query_health(&app, &mock.credit_manager, &token_id_a);
    assert!(!health.above_max_ltv);
    assert!(!health.liquidatable);

    let red_bank_atom_debt = query_red_bank_debt(
        &app,
        &mock.credit_manager,
        &config.red_bank,
        &uatom_info.denom,
    );

    let user_a_debt_shares_atom =
        user_a_borrowed_amount_atom.mul(DEFAULT_DEBT_UNITS_PER_COIN_BORROWED);
    assert_eq!(
        user_a_debt_shares_atom,
        find_by_denom(&uatom_info.denom, &position_a.debt_shares).shares
    );

    let position_b = query_position(&app, &mock.credit_manager, &token_id_b);
    let user_b_debt_shares_atom = user_a_debt_shares_atom
        .multiply_ratio(user_b_borrowed_amount_atom, interim_red_bank_debt.amount);
    assert_eq!(
        user_b_debt_shares_atom,
        find_by_denom(&uatom_info.denom, &position_b.debt_shares).shares
    );

    let red_bank_atom_res: CoinShares = app
        .wrap()
        .query_wasm_smart(
            &mock.credit_manager,
            &QueryMsg::TotalDebtShares(uatom_info.denom.clone()),
        )
        .unwrap();

    assert_eq!(
        red_bank_atom_res.shares,
        user_a_debt_shares_atom + user_b_debt_shares_atom
    );

    let user_a_owed_atom = red_bank_atom_debt
        .amount
        .multiply_ratio(user_a_debt_shares_atom, red_bank_atom_res.shares);
    let user_a_owed_atom_value =
        uatom_info.price * Decimal::from_atomics(user_a_owed_atom, 0).unwrap();

    let osmo_borrowed_amount_dec =
        Decimal::from_atomics(user_a_borrowed_amount_osmo + Uint128::new(1u128), 0).unwrap();
    let osmo_debt_value = uosmo_info.price * osmo_borrowed_amount_dec;

    let total_debt_value = user_a_owed_atom_value.add(osmo_debt_value);
    assert_eq!(health.total_debts_value, total_debt_value);

    let user_a_deposit_amount_osmo_dec =
        Decimal::from_atomics(user_a_deposit_amount_osmo, 0).unwrap();
    let user_a_borrowed_amount_osmo_dec =
        Decimal::from_atomics(user_a_borrowed_amount_osmo, 0).unwrap();
    let user_a_borrowed_amount_atom_dec =
        Decimal::from_atomics(user_a_borrowed_amount_atom, 0).unwrap();

    let lqdt_adjusted_assets_value = (uosmo_info.price
        * user_a_deposit_amount_osmo_dec
        * uosmo_info.liquidation_threshold)
        + (uatom_info.price * user_a_borrowed_amount_atom_dec * uatom_info.liquidation_threshold)
        + (uosmo_info.price * user_a_borrowed_amount_osmo_dec * uosmo_info.liquidation_threshold);
    assert_eq!(
        health.lqdt_health_factor,
        Some(lqdt_adjusted_assets_value.div(total_debt_value))
    );

    let ltv_adjusted_assets_value =
        (uosmo_info.price * user_a_deposit_amount_osmo_dec * uosmo_info.max_ltv)
            + (uatom_info.price * user_a_borrowed_amount_atom_dec * uatom_info.max_ltv)
            + (uosmo_info.price * user_a_borrowed_amount_osmo_dec * uosmo_info.max_ltv);

    assert_eq!(
        health.max_ltv_health_factor,
        Some(ltv_adjusted_assets_value.div(total_debt_value))
    );
}

fn price_change(app: &mut BasicApp, mock: &MockEnv, coin: CoinPrice) {
    app.execute_contract(
        Addr::unchecked("anyone"),
        mock.oracle.clone(),
        &OracleExecuteMsg::ChangePrice(coin),
        &[],
    )
    .unwrap();
}

fn find_by_denom<'a>(denom: &'a str, shares: &'a [DebtSharesValue]) -> &'a DebtSharesValue {
    shares.iter().find(|item| item.denom == *denom).unwrap()
}
