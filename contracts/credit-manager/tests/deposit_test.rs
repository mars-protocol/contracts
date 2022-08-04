extern crate core;

use cosmwasm_std::{Addr, Coin, Uint128};
use cw_multi_test::{App, Executor};

use rover::coins::Coins;
use rover::error::ContractError::{
    ExtraFundsReceived, FundsMismatch, NotTokenOwner, NotWhitelisted,
};
use rover::msg::execute::Action;
use rover::msg::ExecuteMsg;

use crate::helpers::{
    assert_err, get_token_id, mock_app, mock_create_credit_account, query_position,
    setup_credit_manager,
};

pub mod helpers;

#[test]
fn test_only_owner_of_token_can_deposit() {
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

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.assets.len(), 0);

    app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Action::Deposit(coin)],
        },
        &[],
    )
    .unwrap();

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.assets.len(), 0);
}

#[test]
fn test_deposit_but_no_funds() {
    let mut app = mock_app();
    let coin = Coin {
        denom: "uosmo".to_string(),
        amount: Uint128::from(234u128),
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
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Action::Deposit(coin.clone())],
        },
        &[],
    );

    assert_err(
        res,
        FundsMismatch {
            expected: coin.amount,
            received: Uint128::zero(),
        },
    );

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.assets.len(), 0);
}

#[test]
fn test_deposit_but_not_enough_funds() {
    let user = Addr::unchecked("user");
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &user, vec![Coin::new(300u128, "uosmo")])
            .unwrap();
    });

    let coin = Coin {
        denom: "uosmo".to_string(),
        amount: Uint128::from(350u128),
    };
    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![coin.denom.clone()],
        vec![],
    );

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let res = app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Action::Deposit(coin.clone())],
        },
        &[Coin::new(250u128, "uosmo")],
    );

    assert_err(
        res,
        FundsMismatch {
            expected: coin.amount,
            received: Uint128::from(250u128),
        },
    );
}

#[test]
fn test_can_only_deposit_allowed_assets() {
    let user = Addr::unchecked("user");
    let funds = Coin::new(300u128, "uosmo");

    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &user, vec![funds])
            .unwrap();
    });

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec!["ucosmos".to_string()],
        vec![],
    );

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let coin = Coin {
        denom: "uosmo".to_string(),
        amount: Uint128::from(234u128),
    };
    let res = app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Action::Deposit(coin.clone())],
        },
        &[Coin::new(234u128, "uosmo")],
    );

    assert_err(res, NotWhitelisted(coin.denom));

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.assets.len(), 0);
}

#[test]
fn test_extra_funds_received() {
    let user = Addr::unchecked("user");

    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                &user,
                vec![Coin::new(300u128, "uosmo"), Coin::new(50u128, "ucosmos")],
            )
            .unwrap();
    });

    let coin = Coin {
        denom: "uosmo".to_string(),
        amount: Uint128::from(234u128),
    };

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![coin.denom.to_string()],
        vec![],
    );

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let extra_funds = Coin::new(25u128, "ucosmos");
    let res = app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Action::Deposit(coin)],
        },
        &[Coin::new(234u128, "uosmo"), extra_funds.clone()],
    );

    assert_err(res, ExtraFundsReceived(Coins::from(vec![extra_funds])));

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.assets.len(), 0);
}

#[test]
fn test_deposit_success() {
    let user = Addr::unchecked("user");
    let funds = Coin::new(300u128, "uosmo");

    let coin = Coin {
        denom: "uosmo".to_string(),
        amount: Uint128::from(234u128),
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

    app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![Action::Deposit(coin.clone())],
        },
        &[Coin::new(234u128, "uosmo")],
    )
    .unwrap();

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.assets.len(), 1);
    assert_eq!(res.assets.first().unwrap().amount, coin.amount);
    assert_eq!(res.assets.first().unwrap().denom, coin.denom);

    let coin = app
        .wrap()
        .query_balance(mock.credit_manager, "uosmo")
        .unwrap();
    assert_eq!(coin.amount, coin.amount)
}

#[test]
fn test_multiple_deposit_actions() {
    let user = Addr::unchecked("user");
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                &user,
                vec![Coin::new(300u128, "uosmo"), Coin::new(50u128, "ucosmos")],
            )
            .unwrap();
    });

    let coin_a = Coin {
        denom: "uosmo".to_string(),
        amount: Uint128::from(234u128),
    };

    let coin_b = Coin {
        denom: "ucosmos".to_string(),
        amount: Uint128::from(25u128),
    };

    let mock = setup_credit_manager(
        &mut app,
        &Addr::unchecked("owner"),
        vec![coin_a.clone().denom, coin_b.clone().denom],
        vec![],
    );

    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &ExecuteMsg::UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![
                Action::Deposit(coin_a.clone()),
                Action::Deposit(coin_b.clone()),
            ],
        },
        &[Coin::new(234u128, "uosmo"), Coin::new(25u128, "ucosmos")],
    )
    .unwrap();

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.assets.len(), 2);

    let coin = app
        .wrap()
        .query_balance(mock.credit_manager.clone(), "uosmo")
        .unwrap();
    assert_eq!(coin.amount, Uint128::from(234u128));

    let coin = app
        .wrap()
        .query_balance(mock.credit_manager, "ucosmos")
        .unwrap();
    assert_eq!(coin.amount, Uint128::from(25u128));
}
