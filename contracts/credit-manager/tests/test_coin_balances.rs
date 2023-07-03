use cosmwasm_std::{coin, coins, Addr, Uint128};
use cw_multi_test::{BankSudo, SudoMsg};
use mars_rover::{
    error::ContractError,
    msg::execute::{Action::Deposit, CallbackMsg, ChangeExpected},
};

use crate::helpers::{assert_err, uosmo_info, AccountToFund, MockEnv};

pub mod helpers;

#[test]
fn only_rover_can_call_update_coin_balances() {
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.invoke_callback(
        &user,
        CallbackMsg::UpdateCoinBalance {
            account_id,
            previous_balance: coin(1, "utest"),
            change: ChangeExpected::Increase,
        },
    );
    assert_err(res, ContractError::ExternalInvocation)
}

#[test]
fn change_does_not_match_expecations() {
    let osmo_info = uosmo_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[osmo_info.clone()])
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

    // Expected increase, but prev balance was the same
    let res = mock.invoke_callback(
        &mock.rover.clone(),
        CallbackMsg::UpdateCoinBalance {
            account_id: account_id.clone(),
            previous_balance: coin(300, osmo_info.denom.clone()),
            change: ChangeExpected::Increase,
        },
    );
    assert_err(
        res,
        ContractError::BalanceChange {
            denom: "uosmo".to_string(),
            prev_amount: Uint128::new(300),
            curr_amount: Uint128::new(300),
        },
    );

    // Expected increase, but prev balance was higher
    let res = mock.invoke_callback(
        &mock.rover.clone(),
        CallbackMsg::UpdateCoinBalance {
            account_id: account_id.clone(),
            previous_balance: coin(601, osmo_info.denom.clone()),
            change: ChangeExpected::Increase,
        },
    );
    assert_err(
        res,
        ContractError::BalanceChange {
            denom: "uosmo".to_string(),
            prev_amount: Uint128::new(601),
            curr_amount: Uint128::new(300),
        },
    );

    // Expected decrease, but prev balance was the same
    let res = mock.invoke_callback(
        &mock.rover.clone(),
        CallbackMsg::UpdateCoinBalance {
            account_id: account_id.clone(),
            previous_balance: coin(300, osmo_info.denom.clone()),
            change: ChangeExpected::Decrease,
        },
    );
    assert_err(
        res,
        ContractError::BalanceChange {
            denom: "uosmo".to_string(),
            prev_amount: Uint128::new(300),
            curr_amount: Uint128::new(300),
        },
    );

    // Expected decrease, but prev balance was lower
    let res = mock.invoke_callback(
        &mock.rover.clone(),
        CallbackMsg::UpdateCoinBalance {
            account_id,
            previous_balance: coin(250, osmo_info.denom),
            change: ChangeExpected::Decrease,
        },
    );
    assert_err(
        res,
        ContractError::BalanceChange {
            denom: "uosmo".to_string(),
            prev_amount: Uint128::new(250),
            curr_amount: Uint128::new(300),
        },
    );
}

#[test]
fn user_gets_rebalanced_down() {
    let osmo_info = uosmo_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[osmo_info.clone()])
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
            change: ChangeExpected::Decrease,
        },
    )
    .unwrap();

    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.deposits.first().unwrap().denom, osmo_info.denom);
    assert_eq!(position.deposits.first().unwrap().amount.u128(), 100);
}

#[test]
fn user_gets_rebalanced_up() {
    let osmo_info = uosmo_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[osmo_info.clone()])
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
            change: ChangeExpected::Increase,
        },
    )
    .unwrap();

    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.deposits.first().unwrap().denom, osmo_info.denom);
    assert_eq!(position.deposits.first().unwrap().amount.u128(), 500);
}
