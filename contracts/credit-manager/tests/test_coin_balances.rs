use cosmwasm_std::OverflowOperation::Sub;
use cosmwasm_std::{coin, coins, Addr, OverflowError};
use cw_multi_test::{BankSudo, SudoMsg};

use mars_rover::error::ContractError;
use mars_rover::msg::execute::Action::Deposit;
use mars_rover::msg::execute::CallbackMsg;

use crate::helpers::{assert_err, uosmo_info, AccountToFund, MockEnv};

pub mod helpers;

#[test]
fn test_only_rover_can_call_update_coin_balances() {
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.invoke_callback(
        &user,
        CallbackMsg::UpdateCoinBalance {
            account_id,
            previous_balance: coin(1, "utest"),
        },
    );
    assert_err(res, ContractError::ExternalInvocation)
}

#[test]
fn test_user_does_not_have_enough_to_pay_diff() {
    let osmo_info = uosmo_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[osmo_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, osmo_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(osmo_info.to_coin(300))],
        &[osmo_info.to_coin(300)],
    )
    .unwrap();

    let res = mock.invoke_callback(
        &mock.rover.clone(),
        CallbackMsg::UpdateCoinBalance {
            account_id,
            previous_balance: coin(601, osmo_info.denom),
        },
    );

    assert_err(
        res,
        ContractError::Overflow(OverflowError {
            operation: Sub,
            operand1: "300".to_string(),
            operand2: "301".to_string(),
        }),
    )
}

#[test]
fn test_user_gets_rebalanced_down() {
    let osmo_info = uosmo_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[osmo_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, osmo_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(osmo_info.to_coin(300))],
        &[osmo_info.to_coin(300)],
    )
    .unwrap();

    mock.invoke_callback(
        &mock.rover.clone(),
        CallbackMsg::UpdateCoinBalance {
            account_id: account_id.clone(),
            previous_balance: coin(500, osmo_info.denom.clone()),
        },
    )
    .unwrap();

    let position = mock.query_positions(&account_id);
    assert_eq!(position.coins.len(), 1);
    assert_eq!(position.coins.first().unwrap().denom, osmo_info.denom);
    assert_eq!(position.coins.first().unwrap().amount.u128(), 100);
}

#[test]
fn test_user_gets_rebalanced_up() {
    let osmo_info = uosmo_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[osmo_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, osmo_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(osmo_info.to_coin(300))],
        &[osmo_info.to_coin(300)],
    )
    .unwrap();

    mock.app
        .sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: mock.rover.clone().to_string(),
            amount: coins(200, osmo_info.denom.clone()),
        }))
        .unwrap();

    mock.invoke_callback(
        &mock.rover.clone(),
        CallbackMsg::UpdateCoinBalance {
            account_id: account_id.clone(),
            previous_balance: coin(300, osmo_info.denom.clone()),
        },
    )
    .unwrap();

    let position = mock.query_positions(&account_id);
    assert_eq!(position.coins.len(), 1);
    assert_eq!(position.coins.first().unwrap().denom, osmo_info.denom);
    assert_eq!(position.coins.first().unwrap().amount.u128(), 500);
}
