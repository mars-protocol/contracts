extern crate core;

use std::ops::{Add, Div, Mul};

use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_multi_test::{App, BasicApp, Executor};

use credit_manager::borrow::DEFAULT_DEBT_UNITS_PER_COIN_BORROWED;
use mock_oracle::msg::{CoinPrice, ExecuteMsg as OracleExecuteMsg};
use mock_red_bank::msg::QueryMsg::UserAssetDebt;
use mock_red_bank::msg::UserAssetDebtResponse;
use rover::error::ContractError::AccountUnhealthy;
use rover::msg::execute::Action::{Borrow, Deposit};
use rover::msg::ExecuteMsg;

use crate::helpers::{
    assert_err, fund_red_bank, get_token_id, mock_app, mock_create_credit_account, query_config,
    query_position, setup_credit_manager, CoinPriceLTV, MockEnv,
};

pub mod helpers;

#[test]
fn test_only_assets_with_no_debts() {
    let user = Addr::unchecked("user");
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &user, vec![Coin::new(300u128, "uosmo")])
            .unwrap();
    });

    let coin_info = CoinPriceLTV {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(25u128, 2).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
    };

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![coin_info.clone()],
        vec![],
    );

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let deposit_amount = Uint128::from(234u128);

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

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.token_id, token_id);
    assert_eq!(res.assets.len(), 1);
    assert_eq!(res.debt_shares.len(), 0);

    let assets_value = coin_info.price * Decimal::from_atomics(deposit_amount, 0).unwrap();
    assert_eq!(res.assets_value, assets_value);
    assert_eq!(
        res.ltv_adjusted_assets_value,
        assets_value * coin_info.max_ltv
    );

    assert_eq!(res.debts_value, Decimal::zero());
    assert_eq!(res.health_factor, None);
    assert_eq!(res.healthy, true);
}

#[test]
fn test_terra_ragnarok() {
    // Assets drop in value to zero with zero debt value but debt shares outstanding
    let user = Addr::unchecked("user");
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &user, vec![Coin::new(300u128, "uluna")])
            .unwrap();
    });

    let coin_info = CoinPriceLTV {
        denom: "uluna".to_string(),
        price: Decimal::from_atomics(100u128, 0).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
    };

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![coin_info.clone()],
        vec![],
    );

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let config = query_config(&mut app, &mock.credit_manager.clone());

    fund_red_bank(
        &mut app,
        config.red_bank.clone(),
        vec![Coin::new(1000u128, "uluna")],
    );

    let deposit_amount = Uint128::from(234u128);

    app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![
                Deposit(coin_info.to_coin(deposit_amount)),
                Borrow(coin_info.to_coin(Uint128::from(42u128))),
            ],
        },
        &[Coin::new(deposit_amount.into(), "uluna")],
    )
    .unwrap();

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.healthy, true);

    price_change(
        &mut app,
        &mock,
        CoinPrice {
            denom: coin_info.denom,
            price: Decimal::zero(),
        },
    );

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.assets.len(), 1);
    assert!(res.debt_shares.len() > 0);
    assert_eq!(res.assets_value, Decimal::zero());
    assert_eq!(res.ltv_adjusted_assets_value, Decimal::zero());
    assert_eq!(res.debts_value, Decimal::zero());
    assert_eq!(res.health_factor, None);
    assert_eq!(res.healthy, false);
}

#[test]
fn test_debts_no_assets() {
    let user = Addr::unchecked("user");
    let mut app = mock_app();

    let coin_info = CoinPriceLTV {
        denom: "uosmo".to_string(),
        price: Decimal::one(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
    };

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![coin_info.clone()],
        vec![],
    );
    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let config = query_config(&mut app, &mock.credit_manager.clone());

    fund_red_bank(
        &mut app,
        config.red_bank.clone(),
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

    let borrowed_amount_dec = Decimal::from_atomics(borrowed_amount, 0).unwrap();
    let value_of_debt = coin_info.price * borrowed_amount_dec + Decimal::one(); // Simulated interest from mock_red_bank == 1
    let ltv_adjusted_value_of_assets = coin_info.price * borrowed_amount_dec * coin_info.max_ltv;
    assert_err(
        res,
        AccountUnhealthy {
            health_factor: ltv_adjusted_value_of_assets.div(value_of_debt).to_string(),
        },
    );

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.token_id, token_id);
    assert_eq!(res.assets.len(), 0);
    assert_eq!(res.debt_shares.len(), 0);
    assert_eq!(res.assets_value, Decimal::zero());
    assert_eq!(res.ltv_adjusted_assets_value, Decimal::zero());
    assert_eq!(res.debts_value, Decimal::zero());
    assert_eq!(res.health_factor, None);
    assert_eq!(res.healthy, true);
}

#[test]
fn no_assets_no_debt_value_but_shares_outstanding() {
    let user = Addr::unchecked("user");
    let mut app = mock_app();

    let coin_info = CoinPriceLTV {
        denom: "junkcoin".to_string(),
        price: Decimal::zero(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
    };

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![coin_info.clone()],
        vec![],
    );
    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let config = query_config(&mut app, &mock.credit_manager.clone());

    fund_red_bank(
        &mut app,
        config.red_bank.clone(),
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

    assert_err(
        res,
        AccountUnhealthy {
            health_factor: "n/a".to_string(),
        },
    );

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.token_id, token_id);
    assert_eq!(res.assets.len(), 0);
    assert_eq!(res.debt_shares.len(), 0);
    assert_eq!(res.assets_value, Decimal::zero());
    assert_eq!(res.ltv_adjusted_assets_value, Decimal::zero());
    assert_eq!(res.debts_value, Decimal::zero());
    assert_eq!(res.health_factor, None);
    assert_eq!(res.healthy, true);
}

#[test]
fn test_assets_and_ltv_adjusted_value() {
    let user = Addr::unchecked("user");
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &user, vec![Coin::new(300u128, "uosmo")])
            .unwrap();
    });

    let uosmo_info = CoinPriceLTV {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(5265478965412365487125u128, 12).unwrap(),
        max_ltv: Decimal::from_atomics(3u128, 1).unwrap(),
    };

    let uatom_info = CoinPriceLTV {
        denom: "uatom".to_string(),
        price: Decimal::from_atomics(7012302005u128, 3).unwrap(),
        max_ltv: Decimal::from_atomics(8u128, 1).unwrap(),
    };

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![uosmo_info.clone(), uatom_info.clone()],
        vec![],
    );

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let config = query_config(&mut app, &mock.credit_manager.clone());

    fund_red_bank(
        &mut app,
        config.red_bank.clone(),
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
                Borrow(uatom_info.to_coin(deposit_amount)),
            ],
        },
        &[Coin::new(deposit_amount.into(), "uosmo")],
    )
    .unwrap();

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.token_id, token_id);
    assert_eq!(res.assets.len(), 2);

    let borrowed_amount_dec = Decimal::from_atomics(borrowed_amount, 0).unwrap();
    let deposit_amount_dec = Decimal::from_atomics(deposit_amount, 0).unwrap();
    assert_eq!(
        res.assets_value,
        uosmo_info.price * deposit_amount_dec + uatom_info.price * borrowed_amount_dec
    );
    let ltv_adjusted_assets_value = uosmo_info.price * deposit_amount_dec * uosmo_info.max_ltv
        + uatom_info.price * borrowed_amount_dec * uatom_info.max_ltv;
    assert_eq!(res.ltv_adjusted_assets_value, ltv_adjusted_assets_value);

    assert_eq!(
        res.health_factor.unwrap(),
        ltv_adjusted_assets_value.div(uatom_info.price.mul(borrowed_amount_dec + Decimal::one()))
    );
    assert_eq!(res.healthy, true);
}

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

    let uosmo_info = CoinPriceLTV {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(5265478965412365487125u128, 12).unwrap(),
        max_ltv: Decimal::from_atomics(3u128, 1).unwrap(),
    };

    let uatom_info = CoinPriceLTV {
        denom: "uatom".to_string(),
        price: Decimal::from_atomics(7012302005u128, 3).unwrap(),
        max_ltv: Decimal::from_atomics(8u128, 1).unwrap(),
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

    let config = query_config(&mut app, &mock.credit_manager.clone());

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

    let interim_red_bank_debt: UserAssetDebtResponse = app
        .wrap()
        .query_wasm_smart(
            config.red_bank.clone(),
            &UserAssetDebt {
                user_address: mock.credit_manager.clone().into(),
                denom: uatom_info.denom.clone(),
            },
        )
        .unwrap();

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

    let res = query_position(&app, &mock.credit_manager, &token_id_a);
    assert_eq!(res.token_id, token_id_a);
    assert_eq!(res.assets.len(), 2);

    let user_a_borrowed_amount_atom_dec =
        Decimal::from_atomics(user_a_borrowed_amount_atom, 0).unwrap();
    let user_b_borrowed_amount_atom_dec =
        Decimal::from_atomics(user_b_borrowed_amount_atom, 0).unwrap();

    let interest = Decimal::one() + Decimal::one(); // simulated from mock_oracle
    let total_debt_shares_value = uatom_info.price
        * (user_a_borrowed_amount_atom_dec + user_b_borrowed_amount_atom_dec + interest);

    let user_a_debt_shares_atom =
        user_a_borrowed_amount_atom.mul(DEFAULT_DEBT_UNITS_PER_COIN_BORROWED);

    let user_b_debt_shares_atom = user_a_debt_shares_atom
        .multiply_ratio(user_b_borrowed_amount_atom, interim_red_bank_debt.amount);

    let debt_shares_ownership_ratio = Decimal::checked_from_ratio(
        user_a_debt_shares_atom,
        user_a_debt_shares_atom + user_b_debt_shares_atom,
    )
    .unwrap();

    let atom_debt_value = total_debt_shares_value.mul(debt_shares_ownership_ratio);

    let user_a_borrowed_amount_osmo_dec =
        Decimal::from_atomics(user_a_borrowed_amount_osmo, 0).unwrap();
    let osmo_debt_value = uosmo_info.price * (user_a_borrowed_amount_osmo_dec + Decimal::one());

    let total_debt_value = atom_debt_value.add(osmo_debt_value);
    assert_eq!(res.debts_value, total_debt_value);

    let user_a_deposit_amount_osmo_dec =
        Decimal::from_atomics(user_a_deposit_amount_osmo, 0).unwrap();
    let ltv_adjusted_assets_value =
        (uosmo_info.price * user_a_deposit_amount_osmo_dec * uosmo_info.max_ltv)
            + (uatom_info.price * user_a_borrowed_amount_atom_dec * uatom_info.max_ltv)
            + (uosmo_info.price * user_a_borrowed_amount_osmo_dec * uosmo_info.max_ltv);
    assert_eq!(res.ltv_adjusted_assets_value, ltv_adjusted_assets_value);

    assert_eq!(
        res.health_factor.unwrap(),
        ltv_adjusted_assets_value.div(total_debt_value)
    );
    assert_eq!(res.healthy, true);
}

#[test]
fn test_cannot_borrow_more_than_healthy() {
    let user = Addr::unchecked("user");
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &user, vec![Coin::new(300u128, "uosmo")])
            .unwrap();
    });

    let coin_info = CoinPriceLTV {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(23654u128, 4).unwrap(),
        max_ltv: Decimal::from_atomics(5u128, 1).unwrap(),
    };

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![coin_info.clone()],
        vec![],
    );

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let config = query_config(&mut app, &mock.credit_manager.clone());

    fund_red_bank(
        &mut app,
        config.red_bank.clone(),
        vec![Coin::new(1000u128, "uatom"), Coin::new(1000u128, "uosmo")],
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
        &[Coin::new(Uint128::from(300u128).into(), "uosmo")],
    )
    .unwrap();

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
            actions: vec![Borrow(coin_info.to_coin(Uint128::from(100u128)))],
        },
        &[],
    );

    assert_err(
        res,
        AccountUnhealthy {
            health_factor: "1.086956521739130434".to_string(),
        },
    );
}

fn price_change(app: &mut BasicApp, mock: &MockEnv, coin: CoinPrice) -> () {
    app.execute_contract(
        Addr::unchecked("anyone"),
        mock.oracle.clone(),
        &OracleExecuteMsg::ChangePrice(coin),
        &[],
    )
    .unwrap();
}
