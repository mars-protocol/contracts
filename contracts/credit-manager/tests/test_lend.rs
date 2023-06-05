use std::ops::{Add, Mul};

use cosmwasm_std::{coin, coins, Addr, OverflowError, OverflowOperation, Uint128};
use mars_credit_manager::lend::DEFAULT_LENT_SHARES_PER_COIN;
use mars_rover::{
    error::ContractError,
    msg::execute::Action::{Deposit, Lend},
};

use crate::helpers::{
    assert_err, uosmo_info, AccountToFund, MockEnv, DEFAULT_RED_BANK_COIN_BALANCE,
};

pub mod helpers;

#[test]
fn only_token_owner_can_lend() {
    let coin_info = uosmo_info();
    let owner = Addr::unchecked("owner");
    let mut mock = MockEnv::new().build().unwrap();
    let account_id = mock.create_credit_account(&owner).unwrap();

    let another_user = Addr::unchecked("another_user");
    let res = mock.update_credit_account(
        &account_id,
        &another_user,
        vec![Lend(coin_info.to_coin(12312))],
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
fn can_only_lend_what_is_whitelisted() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().set_params(&[coin_info]).build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res =
        mock.update_credit_account(&account_id, &user, vec![Lend(coin(234, "usomething"))], &[]);

    assert_err(res, ContractError::NotWhitelisted(String::from("usomething")))
}

#[test]
fn lending_zero_raises() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().set_params(&[coin_info.clone()]).build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(&account_id, &user, vec![Lend(coin_info.to_coin(0))], &[]);

    assert_err(res, ContractError::NoAmount)
}

#[test]
fn raises_when_not_enough_assets_to_lend() {
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

    let account_id_a = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id_a,
        &user,
        vec![Deposit(coin_info.to_coin(300)), Lend(coin_info.to_coin(500))],
        &[coin_info.to_coin(300)],
    );

    assert_err(
        res,
        ContractError::Overflow(OverflowError {
            operation: OverflowOperation::Sub,
            operand1: "300".to_string(),
            operand2: "500".to_string(),
        }),
    )
}

#[test]
fn successful_lend() {
    let coin_info = uosmo_info();

    let user_a = Addr::unchecked("user_a");
    let user_b = Addr::unchecked("user_b");

    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user_a.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: user_b.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();

    let account_id_a = mock.create_credit_account(&user_a).unwrap();

    let position = mock.query_positions(&account_id_a);
    assert_eq!(position.deposits.len(), 0);
    assert_eq!(position.lends.len(), 0);

    mock.update_credit_account(
        &account_id_a,
        &user_a,
        vec![Deposit(coin_info.to_coin(300))],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    let red_bank_collateral = mock.query_red_bank_collateral(&coin_info.denom);
    assert_eq!(red_bank_collateral.amount, Uint128::zero());

    mock.update_credit_account(&account_id_a, &user_a, vec![Lend(coin_info.to_coin(50))], &[])
        .unwrap();

    // Assert deposits decreased
    let position = mock.query_positions(&account_id_a);
    assert_eq!(position.deposits.len(), 1);
    let deposit_res = position.deposits.first().unwrap();
    let expected_net_deposit_amount = Uint128::new(250); // Deposit - Lent
    assert_eq!(deposit_res.amount, expected_net_deposit_amount);

    // Assert lend position amount increased
    let lent_res = position.lends.first().unwrap();
    assert_eq!(position.lends.len(), 1);
    assert_eq!(lent_res.denom, coin_info.denom);
    let lent_amount = Uint128::new(50) + Uint128::new(1); // simulated yield
    assert_eq!(lent_res.amount, lent_amount);
    assert_eq!(lent_res.shares, Uint128::new(50).mul(DEFAULT_LENT_SHARES_PER_COIN));

    // Assert total lent positions increased
    let total = mock.query_total_lent_shares(&coin_info.denom);
    assert_eq!(total.denom, coin_info.denom);
    assert_eq!(total.shares, DEFAULT_LENT_SHARES_PER_COIN.mul(Uint128::new(50)));

    // Assert Rover has indeed sent those tokens to Red Bank
    let balance = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(balance.amount, Uint128::new(250));

    let config = mock.query_config();
    let red_bank_addr = Addr::unchecked(config.red_bank);
    let balance = mock.query_balance(&red_bank_addr, &coin_info.denom);
    assert_eq!(balance.amount, DEFAULT_RED_BANK_COIN_BALANCE.add(Uint128::new(50)));

    // Assert Rover's collateral balance in Red bank
    let red_bank_collateral = mock.query_red_bank_collateral(&coin_info.denom);
    assert_eq!(red_bank_collateral.amount, lent_amount);

    // Second user comes and performs a lend
    let account_id_b = mock.create_credit_account(&user_b).unwrap();
    mock.update_credit_account(
        &account_id_b,
        &user_b,
        vec![Deposit(coin_info.to_coin(300)), Lend(coin_info.to_coin(50))],
        &[coin(300, coin_info.denom)],
    )
    .unwrap();

    // Assert lend position shares amount is proportionally right given existing participant in pool
    let position = mock.query_positions(&account_id_b);
    let expected_shares = total.shares.multiply_ratio(Uint128::new(50), red_bank_collateral.amount);
    assert_eq!(position.lends.first().unwrap().shares, expected_shares);
}
