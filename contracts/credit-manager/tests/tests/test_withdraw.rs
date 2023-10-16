use cosmwasm_std::{coin, coins, Addr, Coin, OverflowError, OverflowOperation::Sub, Uint128};
use mars_credit_manager::error::{ContractError, ContractError::NotTokenOwner};
use mars_types::credit_manager::Action;

use super::helpers::{assert_err, uatom_info, uosmo_info, AccountToFund, MockEnv};

#[test]
fn only_owner_of_token_can_withdraw() {
    let coin_info = uosmo_info();
    let owner = Addr::unchecked("owner");
    let mut mock = MockEnv::new().build().unwrap();
    let account_id = mock.create_credit_account(&owner).unwrap();

    let another_user = Addr::unchecked("another_user");
    let res = mock.update_credit_account(
        &account_id,
        &another_user,
        vec![Action::Withdraw(coin_info.to_action_coin(382))],
        &[],
    );

    assert_err(
        res,
        NotTokenOwner {
            user: another_user.into(),
            account_id: account_id.clone(),
        },
    );

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);
}

#[test]
fn withdraw_nothing() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().set_params(&[coin_info.clone()]).build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::Withdraw(coin_info.to_action_coin(0))],
        &[],
    );

    assert_err(res, ContractError::NoAmount);
    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);
}

#[test]
fn withdraw_but_no_funds() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().set_params(&[coin_info.clone()]).build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::Withdraw(coin_info.to_action_coin(234))],
        &[],
    );

    assert_err(
        res,
        ContractError::Overflow(OverflowError {
            operation: Sub,
            operand1: "0".to_string(),
            operand2: "234".to_string(),
        }),
    );

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);
}

#[test]
fn withdraw_but_not_enough_funds() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Action::Deposit(coin_info.to_coin(300)),
            Action::Withdraw(coin_info.to_action_coin(400)),
        ],
        &[coin(300, coin_info.denom)],
    );

    assert_err(
        res,
        ContractError::Overflow(OverflowError {
            operation: Sub,
            operand1: "300".to_string(),
            operand2: "400".to_string(),
        }),
    );

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);
}

#[test]
fn cannot_withdraw_more_than_healthy() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Action::Deposit(coin_info.to_coin(200)),
            Action::Borrow(coin_info.to_coin(400)),
            Action::Withdraw(coin_info.to_action_coin(50)),
        ],
        &[coin(200, coin_info.denom)],
    );

    assert_err(
        res,
        ContractError::AboveMaxLTV {
            account_id: account_id.clone(),
            max_ltv_health_factor: "0.940594059405940594".to_string(),
        },
    );

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);
}

#[test]
fn withdraw_success() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let deposit_amount = 234;
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Action::Deposit(coin_info.to_coin(deposit_amount)),
            Action::Withdraw(coin_info.to_action_coin(deposit_amount)),
        ],
        &[Coin::new(deposit_amount, coin_info.denom.clone())],
    )
    .unwrap();

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);

    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, Uint128::zero())
}

#[test]
fn withdraw_account_balance() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let deposit_amount = 234;
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Action::Deposit(coin_info.to_coin(deposit_amount)),
            Action::Withdraw(coin_info.to_action_coin_full_balance()),
        ],
        &[Coin::new(deposit_amount, coin_info.denom.clone())],
    )
    .unwrap();

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);

    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, Uint128::zero())
}

#[test]
fn multiple_withdraw_actions() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(234, uosmo_info.denom.clone()), coin(25, uatom_info.denom.clone())],
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let uosmo_amount = Uint128::new(234);
    let uatom_amount = Uint128::new(25);

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Action::Deposit(uosmo_info.to_coin(uosmo_amount.u128())),
            Action::Deposit(uatom_info.to_coin(uatom_amount.u128())),
        ],
        &[coin(234, uosmo_info.denom.clone()), coin(25, uatom_info.denom.clone())],
    )
    .unwrap();

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 2);

    let coin = mock.query_balance(&user, &uosmo_info.denom);
    assert_eq!(coin.amount, Uint128::zero());

    let coin = mock.query_balance(&user, &uatom_info.denom);
    assert_eq!(coin.amount, Uint128::zero());

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::Withdraw(uosmo_info.to_action_coin(uosmo_amount.u128()))],
        &[],
    )
    .unwrap();

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 1);

    let coin = mock.query_balance(&mock.rover, &uosmo_info.denom);
    assert_eq!(coin.amount, Uint128::zero());

    let coin = mock.query_balance(&user, &uosmo_info.denom);
    assert_eq!(coin.amount, uosmo_amount);

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::Withdraw(uatom_info.to_action_coin(20))],
        &[],
    )
    .unwrap();

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 1);

    let coin = mock.query_balance(&mock.rover, &uatom_info.denom);
    assert_eq!(coin.amount, Uint128::new(5));

    let coin = mock.query_balance(&user, &uatom_info.denom);
    assert_eq!(coin.amount, Uint128::new(20));

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::Withdraw(uatom_info.to_action_coin(5))],
        &[],
    )
    .unwrap();

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);

    let coin = mock.query_balance(&mock.rover, &uatom_info.denom);
    assert_eq!(coin.amount, Uint128::zero());

    let coin = mock.query_balance(&user, &uatom_info.denom);
    assert_eq!(coin.amount, uatom_amount);
}
