use std::ops::Add;

use cosmwasm_std::{coin, coins, Addr, Coin, OverflowError, OverflowOperation, Uint128};
use mars_rover::{
    error::ContractError,
    msg::execute::{
        Action::{Deposit, Lend},
        ActionAmount, ActionCoin,
    },
};

use crate::helpers::{
    assert_err, blacklisted_coin, coin_info, uosmo_info, AccountToFund, MockEnv,
    DEFAULT_RED_BANK_COIN_BALANCE,
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
        vec![Lend(coin_info.to_action_coin(12312))],
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
    let coin_info = blacklisted_coin();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().set_params(&[coin_info.clone()]).build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Lend(coin_info.to_action_coin(50))],
        &[],
    );

    assert_err(res, ContractError::NotWhitelisted(String::from("uluna")))
}

#[test]
fn lending_zero_raises() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().set_params(&[coin_info.clone()]).build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Lend(coin_info.to_action_coin(0))],
        &[],
    );

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
        vec![Deposit(coin_info.to_coin(300)), Lend(coin_info.to_action_coin(500))],
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
fn raises_when_attempting_to_lend_account_balance_with_no_funds() {
    let coin_info = uosmo_info();

    let user_a = Addr::unchecked("user_a");

    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user_a.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();

    let account_id_a = mock.create_credit_account(&user_a).unwrap();

    let position = mock.query_positions(&account_id_a);
    assert_eq!(position.deposits.len(), 0);
    assert_eq!(position.lends.len(), 0);

    let red_bank_collateral = mock.query_red_bank_collateral(&account_id_a, &coin_info.denom);
    assert_eq!(red_bank_collateral.amount, Uint128::zero());

    let res = mock.update_credit_account(
        &account_id_a,
        &user_a,
        vec![Lend(ActionCoin {
            denom: "uosmo".to_string(),
            amount: ActionAmount::AccountBalance,
        })],
        &[],
    );

    assert_err(res, ContractError::NoAmount)
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

    let red_bank_collateral = mock.query_red_bank_collateral(&account_id_a, &coin_info.denom);
    assert_eq!(red_bank_collateral.amount, Uint128::zero());

    mock.update_credit_account(
        &account_id_a,
        &user_a,
        vec![Lend(coin_info.to_action_coin(50))],
        &[],
    )
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

    // Assert Rover has indeed sent those tokens to Red Bank
    let balance = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(balance.amount, Uint128::new(250));

    let config = mock.query_config();
    let red_bank_addr = Addr::unchecked(config.red_bank);
    let balance = mock.query_balance(&red_bank_addr, &coin_info.denom);
    assert_eq!(balance.amount, DEFAULT_RED_BANK_COIN_BALANCE.add(Uint128::new(50)));

    // Assert Rover's collateral balance in Red bank
    let red_bank_collateral = mock.query_red_bank_collateral(&account_id_a, &coin_info.denom);
    assert_eq!(red_bank_collateral.amount, lent_amount);

    // Second user comes and performs a lend
    let account_id_b = mock.create_credit_account(&user_b).unwrap();
    mock.update_credit_account(
        &account_id_b,
        &user_b,
        vec![Deposit(coin_info.to_coin(300)), Lend(coin_info.to_action_coin(50))],
        &[coin(300, coin_info.denom)],
    )
    .unwrap();
}

#[test]
fn successful_account_balance_lend() {
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

    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 0);
    assert_eq!(position.lends.len(), 0);

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(coin_info.to_coin(300))],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    let red_bank_collateral = mock.query_red_bank_collateral(&account_id, &coin_info.denom);
    assert_eq!(red_bank_collateral.amount, Uint128::zero());

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Lend(ActionCoin {
            denom: "uosmo".to_string(),
            amount: ActionAmount::AccountBalance,
        })],
        &[],
    )
    .unwrap();

    // Assert deposits decreased
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 0);

    // Assert lend position amount increased
    let lent_res = position.lends.first().unwrap();
    assert_eq!(position.lends.len(), 1);
    assert_eq!(lent_res.denom, coin_info.denom);
    let lent_amount = Uint128::new(300) + Uint128::new(1); // account balance + simulated yield
    assert_eq!(lent_res.amount, lent_amount);

    // Assert Rover has indeed sent those tokens to Red Bank
    let balance = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(balance.amount, Uint128::new(0)); //total deposited minus account balance -> 300-300=0

    let config = mock.query_config();
    let red_bank_addr = Addr::unchecked(config.red_bank);
    let balance = mock.query_balance(&red_bank_addr, &coin_info.denom);
    assert_eq!(balance.amount, DEFAULT_RED_BANK_COIN_BALANCE.add(Uint128::new(300)));

    // Assert Rover's collateral balance in Red bank
    let red_bank_collateral = mock.query_red_bank_collateral(&account_id, &coin_info.denom);
    assert_eq!(red_bank_collateral.amount, lent_amount);
}

#[test]
fn query_positions_successfully_with_paginated_lends() {
    let coins_info = vec![
        coin_info("coin_1"),
        coin_info("coin_2"),
        coin_info("coin_123"),
        coin_info("coin_4"),
        coin_info("coin_5"),
        coin_info("coin_11"),
        coin_info("coin_7"),
    ];

    let user = Addr::unchecked("user");

    let funded_amt = 300u128;

    let coins: Vec<_> = coins_info.iter().map(|coin| coin.to_coin(funded_amt)).collect();
    let mut mock = MockEnv::new()
        .set_params(&coins_info)
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins.clone(),
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 0);
    assert_eq!(position.lends.len(), 0);

    for coin in coins.iter() {
        mock.update_credit_account(
            &account_id,
            &user,
            vec![Deposit(coin.clone())],
            &[coin.clone()],
        )
        .unwrap();

        mock.update_credit_account(
            &account_id,
            &user,
            vec![Lend(ActionCoin {
                denom: coin.denom.clone(),
                amount: ActionAmount::AccountBalance,
            })],
            &[],
        )
        .unwrap();
    }

    // Assert deposits decreased
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 0);

    // Assert lends increased
    assert_eq!(position.lends.len(), coins.len());
    let expected_amt = funded_amt + 1; // account balance + simulated yield
    assert_eq!(position.lends[0].clone(), Coin::new(expected_amt, "coin_1"));
    assert_eq!(position.lends[1].clone(), Coin::new(expected_amt, "coin_2"));
    assert_eq!(position.lends[2].clone(), Coin::new(expected_amt, "coin_123"));
    assert_eq!(position.lends[3].clone(), Coin::new(expected_amt, "coin_4"));
    assert_eq!(position.lends[4].clone(), Coin::new(expected_amt, "coin_5"));
    assert_eq!(position.lends[5].clone(), Coin::new(expected_amt, "coin_11"));
    assert_eq!(position.lends[6].clone(), Coin::new(expected_amt, "coin_7"));
}
