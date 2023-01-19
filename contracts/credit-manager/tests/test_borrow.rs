use std::ops::{Mul, Sub};

use cosmwasm_std::{coin, coins, Addr, Uint128};
use mars_credit_manager::borrow::DEFAULT_DEBT_SHARES_PER_COIN_BORROWED;
use mars_rover::{
    error::ContractError,
    msg::execute::Action::{Borrow, Deposit},
};

use crate::helpers::{
    assert_err, uosmo_info, AccountToFund, MockEnv, DEFAULT_RED_BANK_COIN_BALANCE,
};

pub mod helpers;

#[test]
fn only_token_owner_can_borrow() {
    let coin_info = uosmo_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().allowed_coins(&[coin_info.clone()]).build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let another_user = Addr::unchecked("another_user");
    let res = mock.update_credit_account(
        &account_id,
        &another_user,
        vec![Borrow(coin_info.to_coin(12312))],
        &[],
    );

    assert_err(
        res,
        ContractError::NotTokenOwner {
            user: another_user.into(),
            account_id,
        },
    )
}

#[test]
fn can_only_borrow_what_is_whitelisted() {
    let coin_info = uosmo_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().allowed_coins(&[coin_info]).build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res =
        mock.update_credit_account(&account_id, &user, vec![Borrow(coin(234, "usomething"))], &[]);

    assert_err(res, ContractError::NotWhitelisted(String::from("usomething")))
}

#[test]
fn borrowing_zero_does_nothing() {
    let coin_info = uosmo_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().allowed_coins(&[coin_info.clone()]).build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res =
        mock.update_credit_account(&account_id, &user, vec![Borrow(coin_info.to_coin(0))], &[]);

    assert_err(res, ContractError::NoAmount);

    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 0);
    assert_eq!(position.debts.len(), 0);
}

#[test]
fn cannot_borrow_above_max_ltv() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 0);
    assert_eq!(position.debts.len(), 0);

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(coin_info.to_coin(300)), Borrow(coin_info.to_coin(800))],
        &[coin(300, coin_info.denom)],
    );

    assert_err(
        res,
        ContractError::AboveMaxLTV {
            account_id,
            max_ltv_health_factor: "0.96".to_string(),
        },
    );
}

#[test]
fn success_when_new_debt_asset() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 0);
    assert_eq!(position.debts.len(), 0);
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(coin(300, coin_info.denom.clone())),
            Borrow(coin(42, coin_info.denom.clone())),
        ],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    let asset_res = position.deposits.first().unwrap();
    assert_eq!(
        asset_res.amount,
        Uint128::new(342) // Deposit + Borrow
    );
    assert_eq!(asset_res.denom, coin_info.denom);

    let debt_shares_res = position.debts.first().unwrap();
    assert_eq!(position.debts.len(), 1);
    assert_eq!(debt_shares_res.shares, Uint128::new(42).mul(DEFAULT_DEBT_SHARES_PER_COIN_BORROWED));
    assert_eq!(debt_shares_res.denom, coin_info.denom);
    let debt_amount = Uint128::new(42) + Uint128::new(1); // simulated yield
    assert_eq!(debt_shares_res.amount, debt_amount);

    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, Uint128::new(342));

    let config = mock.query_config();
    let coin = mock.query_balance(&Addr::unchecked(config.red_bank), &coin_info.denom);
    assert_eq!(coin.amount, DEFAULT_RED_BANK_COIN_BALANCE.sub(Uint128::new(42)));

    let res = mock.query_total_debt_shares(&coin_info.denom);
    assert_eq!(res.shares, Uint128::new(42).mul(DEFAULT_DEBT_SHARES_PER_COIN_BORROWED));
}

#[test]
fn debt_shares_with_debt_amount() {
    let coin_info = uosmo_info();
    let user_a = Addr::unchecked("user_a");
    let user_b = Addr::unchecked("user_b");
    let mut mock = MockEnv::new()
        .allowed_coins(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user_a.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: user_b.clone(),
            funds: coins(450, coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id_a = mock.create_credit_account(&user_a).unwrap();
    let account_id_b = mock.create_credit_account(&user_b).unwrap();

    mock.update_credit_account(
        &account_id_a,
        &user_a,
        vec![Deposit(coin_info.to_coin(300)), Borrow(coin_info.to_coin(50))],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    let interim_red_bank_debt = mock.query_red_bank_debt(&coin_info.denom);

    mock.update_credit_account(
        &account_id_b,
        &user_b,
        vec![Deposit(coin_info.to_coin(450)), Borrow(coin_info.to_coin(50))],
        &[coin(450, coin_info.denom.clone())],
    )
    .unwrap();

    let token_a_shares = Uint128::new(50).mul(DEFAULT_DEBT_SHARES_PER_COIN_BORROWED);
    let position = mock.query_positions(&account_id_a);
    let debt_position_a = position.debts.first().unwrap();
    assert_eq!(debt_position_a.shares, token_a_shares.clone());
    assert_eq!(debt_position_a.denom, coin_info.denom);

    let token_b_shares = Uint128::new(50)
        .mul(DEFAULT_DEBT_SHARES_PER_COIN_BORROWED)
        .multiply_ratio(Uint128::new(50), interim_red_bank_debt.amount);
    let position = mock.query_positions(&account_id_b);
    let debt_position_b = position.debts.first().unwrap();
    assert_eq!(debt_position_b.shares, token_b_shares.clone());
    assert_eq!(debt_position_b.denom, coin_info.denom);

    let total = mock.query_total_debt_shares(&coin_info.denom);

    assert_eq!(total.shares, debt_position_a.shares + debt_position_b.shares);
}
