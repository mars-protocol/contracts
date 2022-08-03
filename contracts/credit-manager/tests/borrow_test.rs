use std::ops::{Mul, Sub};

use cosmwasm_std::{Addr, Coin, Uint128};
use cw_multi_test::{App, Executor};

use credit_manager::borrow::DEFAULT_DEBT_UNITS_PER_COIN_BORROWED;
use mock_red_bank::msg::QueryMsg::UserAssetDebt;
use mock_red_bank::msg::UserAssetDebtResponse;
use rover::error::ContractError;
use rover::msg::execute::Action::Borrow;
use rover::msg::query::CoinShares;
use rover::msg::ExecuteMsg::UpdateCreditAccount;
use rover::msg::QueryMsg;

use crate::helpers::{
    assert_err, fund_red_bank_native, get_token_id, mock_app, mock_create_credit_account,
    query_config, query_position, setup_credit_manager,
};

pub mod helpers;

#[test]
fn test_only_token_owner_can_borrow() {
    let mut app = mock_app();
    let owner = Addr::unchecked("owner");
    let coin = Coin {
        denom: "uosmo".to_string(),
        amount: Uint128::zero(),
    };

    let mock = setup_credit_manager(&mut app, &owner, vec![coin.denom.clone()], vec![]);
    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &Addr::unchecked("user"))
        .unwrap();
    let token_id = get_token_id(res);

    let another_user = Addr::unchecked("another_user");
    let res = app.execute_contract(
        another_user.clone(),
        mock.credit_manager.clone(),
        &UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Borrow(coin)],
        },
        &[],
    );

    assert_err(
        res,
        ContractError::NotTokenOwner {
            user: another_user.into(),
            token_id,
        },
    )
}

#[test]
fn test_can_only_borrow_what_is_whitelisted() {
    let mut app = mock_app();
    let owner = Addr::unchecked("owner");
    let coin = Coin {
        denom: "usomething".to_string(),
        amount: Uint128::from(234u128),
    };

    let mock = setup_credit_manager(&mut app, &owner, vec!["uosmo".to_string()], vec![]);
    let user = Addr::unchecked("user");
    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let res = app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Borrow(coin)],
        },
        &[],
    );

    assert_err(
        res,
        ContractError::NotWhitelisted(String::from("usomething")),
    )
}

#[test]
fn test_borrowing_zero_does_nothing() {
    let mut app = mock_app();
    let coin = Coin {
        denom: "uosmo".to_string(),
        amount: Uint128::zero(),
    };

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![coin.denom.clone()],
        vec![],
    );
    let user = Addr::unchecked("user");
    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let res = app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Borrow(coin)],
        },
        &[],
    );

    assert_err(res, ContractError::NoAmount);

    let position = query_position(&mut app, &mock.credit_manager, &token_id);
    assert_eq!(position.assets.len(), 0);
    assert_eq!(position.debt_shares.len(), 0);
}

#[test]
fn test_success_when_new_debt_asset() {
    let user = Addr::unchecked("user");
    let funds = Coin::new(300u128, "uosmo");
    let coin = Coin {
        denom: "uosmo".to_string(),
        amount: Uint128::from(42u128),
    };
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &user, vec![funds])
            .unwrap();
    });

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![coin.denom.clone()],
        vec![],
    );
    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let config = query_config(&mut app, &mock.credit_manager.clone());

    fund_red_bank_native(
        &mut app,
        config.red_bank.clone(),
        vec![Coin::new(1000u128, "uosmo")],
    );

    let position = query_position(&mut app, &mock.credit_manager, &token_id);
    assert_eq!(position.assets.len(), 0);
    assert_eq!(position.debt_shares.len(), 0);

    app.execute_contract(
        user,
        mock.credit_manager.clone(),
        &UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Borrow(coin.clone())],
        },
        &[],
    )
    .unwrap();

    let position = query_position(&mut app, &mock.credit_manager, &token_id);
    assert_eq!(position.assets.len(), 1);
    assert_eq!(
        position.assets.first().unwrap().amount,
        Uint128::from(42u128)
    );
    assert_eq!(position.assets.first().unwrap().denom, coin.denom);
    assert_eq!(position.debt_shares.len(), 1);
    assert_eq!(
        position.debt_shares.first().unwrap().shares,
        Uint128::from(42u128).mul(DEFAULT_DEBT_UNITS_PER_COIN_BORROWED)
    );
    assert_eq!(position.debt_shares.first().unwrap().denom, coin.denom);

    let coin = app
        .wrap()
        .query_balance(mock.credit_manager.clone(), "uosmo")
        .unwrap();
    assert_eq!(coin.amount, Uint128::from(42u128));

    let coin = app.wrap().query_balance(config.red_bank, "uosmo").unwrap();
    assert_eq!(
        coin.amount,
        Uint128::from(1000u128).sub(Uint128::from(42u128))
    );

    let res: CoinShares = app
        .wrap()
        .query_wasm_smart(mock.credit_manager, &QueryMsg::TotalDebtShares(coin.denom))
        .unwrap();
    assert_eq!(
        res.shares,
        Uint128::from(42u128).mul(DEFAULT_DEBT_UNITS_PER_COIN_BORROWED)
    );
}

#[test]
fn test_debt_shares_with_debt_amount() {
    let user_a = Addr::unchecked("user_a");
    let user_b = Addr::unchecked("user_b");
    let coin = Coin {
        denom: "uosmo".to_string(),
        amount: Uint128::from(50u128),
    };

    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &user_a, vec![Coin::new(300u128, "uosmo")])
            .unwrap();
        router
            .bank
            .init_balance(storage, &user_b, vec![Coin::new(450u128, "uosmo")])
            .unwrap();
    });

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![coin.denom.clone()],
        vec![],
    );
    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user_a).unwrap();
    let token_id_a = get_token_id(res);
    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user_b).unwrap();
    let token_id_b = get_token_id(res);

    let config = query_config(&mut app, &mock.credit_manager.clone());

    fund_red_bank_native(
        &mut app,
        config.red_bank.clone(),
        vec![Coin::new(1000u128, "uosmo")],
    );

    app.execute_contract(
        user_a,
        mock.credit_manager.clone(),
        &UpdateCreditAccount {
            token_id: token_id_a.clone(),
            actions: vec![Borrow(coin.clone())],
        },
        &[],
    )
    .unwrap();

    let interim_red_bank_debt: UserAssetDebtResponse = app
        .wrap()
        .query_wasm_smart(
            config.red_bank,
            &UserAssetDebt {
                user_address: mock.credit_manager.clone().into(),
                denom: coin.denom.clone(),
            },
        )
        .unwrap();

    app.execute_contract(
        user_b,
        mock.credit_manager.clone(),
        &UpdateCreditAccount {
            token_id: token_id_b.clone(),
            actions: vec![Borrow(coin.clone())],
        },
        &[],
    )
    .unwrap();

    let token_a_shares = Uint128::from(50u128).mul(DEFAULT_DEBT_UNITS_PER_COIN_BORROWED);
    let position = query_position(&mut app, &mock.credit_manager, &token_id_a);
    assert_eq!(
        position.debt_shares.first().unwrap().shares,
        token_a_shares.clone()
    );

    let token_b_shares = Uint128::from(50u128)
        .mul(DEFAULT_DEBT_UNITS_PER_COIN_BORROWED)
        .multiply_ratio(Uint128::from(50u128), interim_red_bank_debt.amount);

    let position = query_position(&mut app, &mock.credit_manager, &token_id_b);
    assert_eq!(
        position.debt_shares.first().unwrap().shares,
        token_b_shares.clone()
    );

    let res: CoinShares = app
        .wrap()
        .query_wasm_smart(mock.credit_manager, &QueryMsg::TotalDebtShares(coin.denom))
        .unwrap();
    assert_eq!(res.shares, token_a_shares + token_b_shares);
}
