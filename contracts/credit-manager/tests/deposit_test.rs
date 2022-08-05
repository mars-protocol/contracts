extern crate core;

use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_multi_test::{App, Executor};

use rover::coins::Coins;
use rover::error::ContractError::{
    ExtraFundsReceived, FundsMismatch, NotTokenOwner, NotWhitelisted,
};
use rover::msg::execute::Action;
use rover::msg::query::PositionResponse;
use rover::msg::ExecuteMsg;

use crate::helpers::{
    assert_err, get_token_id, mock_app, mock_create_credit_account, query_position,
    setup_credit_manager, CoinInfo,
};

pub mod helpers;

// TODO: Assert values

#[test]
fn test_only_owner_of_token_can_deposit() {
    let mut app = mock_app();
    let coin = Coin {
        denom: "uosmo".to_string(),
        amount: Uint128::zero(),
    };

    let mock = setup_credit_manager(&mut app, &Addr::unchecked("owner"), vec![], vec![]);

    let user = Addr::unchecked("user");
    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let another_user = Addr::unchecked("another_user");
    let res = app.execute_contract(
        another_user.clone(),
        mock.credit_manager,
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Action::Deposit(coin)],
        },
        &[],
    );

    assert_err(
        res,
        NotTokenOwner {
            user: another_user.into(),
            token_id,
        },
    )
}

#[test]
fn test_deposit_nothing() {
    let mut app = mock_app();
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

    let user = Addr::unchecked("user");
    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.coin_assets.len(), 0);

    app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Action::Deposit(coin_info.to_coin(Uint128::zero()))],
        },
        &[],
    )
    .unwrap();

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.coin_assets.len(), 0);
}

#[test]
fn test_deposit_but_no_funds() {
    let mut app = mock_app();
    let coin_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(25u128, 2).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
    };
    let deposit_amount = Uint128::from(234u128);
    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![coin_info.clone()],
        vec![],
    );

    let user = Addr::unchecked("user");
    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let res = app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Action::Deposit(coin_info.to_coin(deposit_amount.clone()))],
        },
        &[],
    );

    assert_err(
        res,
        FundsMismatch {
            expected: deposit_amount,
            received: Uint128::zero(),
        },
    );

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.coin_assets.len(), 0);
}

#[test]
fn test_deposit_but_not_enough_funds() {
    let user = Addr::unchecked("user");
    let coin_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(25u128, 2).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
    };
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                &user,
                vec![Coin::new(300u128, coin_info.denom.clone())],
            )
            .unwrap();
    });

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![coin_info.clone()],
        vec![],
    );

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let res = app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Action::Deposit(coin_info.to_coin(Uint128::from(350u128)))],
        },
        &[Coin::new(250u128, coin_info.denom.clone())],
    );

    assert_err(
        res,
        FundsMismatch {
            expected: Uint128::from(350u128),
            received: Uint128::from(250u128),
        },
    );
}

#[test]
fn test_can_only_deposit_allowed_assets() {
    let user = Addr::unchecked("user");
    let coin_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(25u128, 2).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
    };
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                &user,
                vec![Coin::new(300u128, coin_info.denom.clone())],
            )
            .unwrap();
    });

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![coin_info.clone()],
        vec![],
    );

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let not_allowed_coin = Coin {
        denom: "ujakecoin".to_string(),
        amount: Uint128::from(234u128),
    };
    let res = app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Action::Deposit(not_allowed_coin.clone())],
        },
        &[Coin::new(234u128, coin_info.denom.clone())],
    );

    assert_err(res, NotWhitelisted(not_allowed_coin.denom));

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.coin_assets.len(), 0);
}

#[test]
fn test_extra_funds_received() {
    let user = Addr::unchecked("user");
    let uosmo_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(25u128, 2).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
    };
    let uatom_info = CoinInfo {
        denom: "uatom".to_string(),
        price: Decimal::from_atomics(10u128, 1).unwrap(),
        max_ltv: Decimal::from_atomics(82u128, 2).unwrap(),
        liquidation_threshold: Decimal::from_atomics(9u128, 1).unwrap(),
    };
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                &user,
                vec![
                    Coin::new(300u128, uosmo_info.denom.clone()),
                    Coin::new(250u128, uatom_info.denom.clone()),
                ],
            )
            .unwrap();
    });

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![uosmo_info.clone(), uatom_info.clone()],
        vec![],
    );

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let extra_funds = Coin::new(25u128, uatom_info.denom.clone());
    let res = app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Action::Deposit(uosmo_info.to_coin(Uint128::from(234u128)))],
        },
        &[
            Coin::new(234u128, uosmo_info.denom.clone()),
            extra_funds.clone(),
        ],
    );

    assert_err(res, ExtraFundsReceived(Coins::from(vec![extra_funds])));

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.coin_assets.len(), 0);
}

#[test]
fn test_deposit_success() {
    let user = Addr::unchecked("user");
    let coin_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(25u128, 2).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
    };

    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                &user,
                vec![Coin::new(300u128, coin_info.denom.clone())],
            )
            .unwrap();
    });
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
            actions: vec![Action::Deposit(coin_info.to_coin(deposit_amount))],
        },
        &[Coin::new(deposit_amount.into(), coin_info.denom.clone())],
    )
    .unwrap();

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.coin_assets.len(), 1);
    assert_eq!(res.coin_assets.first().unwrap().amount, deposit_amount);
    assert_eq!(res.coin_assets.first().unwrap().denom, coin_info.denom);
    assert_eq!(res.coin_assets.first().unwrap().price, coin_info.price);

    let coin = app
        .wrap()
        .query_balance(mock.credit_manager, coin_info.denom.clone())
        .unwrap();
    assert_eq!(coin.amount, deposit_amount)
}

#[test]
fn test_multiple_deposit_actions() {
    let user = Addr::unchecked("user");
    let uosmo_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(25u128, 2).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
    };
    let uatom_info = CoinInfo {
        denom: "uatom".to_string(),
        price: Decimal::from_atomics(10u128, 1).unwrap(),
        max_ltv: Decimal::from_atomics(82u128, 2).unwrap(),
        liquidation_threshold: Decimal::from_atomics(9u128, 1).unwrap(),
    };
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                &user,
                vec![
                    Coin::new(300u128, uosmo_info.denom.clone()),
                    Coin::new(50u128, uatom_info.denom.clone()),
                ],
            )
            .unwrap();
    });

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![uosmo_info.clone(), uatom_info.clone()],
        vec![],
    );

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let uosmo_amount = Uint128::from(234u128);
    let uatom_amount = Uint128::from(25u128);

    app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![
                Action::Deposit(uosmo_info.to_coin(uosmo_amount)),
                Action::Deposit(uatom_info.to_coin(uatom_amount)),
            ],
        },
        &[
            Coin::new(234u128, uosmo_info.denom.clone()),
            Coin::new(25u128, uatom_info.denom.clone()),
        ],
    )
    .unwrap();

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.coin_assets.len(), 2);
    assert_present(&res, &uosmo_info, uosmo_amount);
    assert_present(&res, &uatom_info, uatom_amount);

    let coin = app
        .wrap()
        .query_balance(mock.credit_manager.clone(), uosmo_info.denom.clone())
        .unwrap();
    assert_eq!(coin.amount, uosmo_amount);

    let coin = app
        .wrap()
        .query_balance(mock.credit_manager, "uatom")
        .unwrap();
    assert_eq!(coin.amount, uatom_amount);
}

fn assert_present(res: &PositionResponse, coin: &CoinInfo, amount: Uint128) {
    res.coin_assets
        .iter()
        .find(|item| item.denom == coin.denom && &item.amount == &amount)
        .unwrap();
}
