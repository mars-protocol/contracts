use cosmwasm_std::OverflowOperation::Sub;
use cosmwasm_std::{coin, coins, Addr, OverflowError};
use cw_multi_test::{BankSudo, SudoMsg};

use rover::error::ContractError;
use rover::msg::execute::Action::Deposit;
use rover::msg::execute::CallbackMsg;

use crate::helpers::{assert_err, get_coin, uatom_info, uosmo_info, AccountToFund, MockEnv};

pub mod helpers;

#[test]
fn test_only_rover_can_call_update_coin_balances() {
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.invoke_callback(
        &user,
        CallbackMsg::UpdateCoinBalances {
            account_id,
            previous_balances: vec![],
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
        CallbackMsg::UpdateCoinBalances {
            account_id,
            previous_balances: coins(601, osmo_info.denom),
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
        CallbackMsg::UpdateCoinBalances {
            account_id: account_id.clone(),
            previous_balances: coins(500, osmo_info.denom.clone()),
        },
    )
    .unwrap();

    let position = mock.query_position(&account_id);
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
        CallbackMsg::UpdateCoinBalances {
            account_id: account_id.clone(),
            previous_balances: coins(300, osmo_info.denom.clone()),
        },
    )
    .unwrap();

    let position = mock.query_position(&account_id);
    assert_eq!(position.coins.len(), 1);
    assert_eq!(position.coins.first().unwrap().denom, osmo_info.denom);
    assert_eq!(position.coins.first().unwrap().amount.u128(), 500);
}

#[test]
fn test_works_on_multiple() {
    let osmo_info = uosmo_info();
    let atom_info = uatom_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[osmo_info.clone(), atom_info.clone()])
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    mock.app
        .sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: mock.rover.clone().to_string(),
            amount: vec![
                coin(143, osmo_info.denom.clone()),
                coin(57, atom_info.denom.clone()),
            ],
        }))
        .unwrap();

    mock.invoke_callback(
        &mock.rover.clone(),
        CallbackMsg::UpdateCoinBalances {
            account_id: account_id.clone(),
            previous_balances: vec![coin(0, osmo_info.denom), coin(0, atom_info.denom)],
        },
    )
    .unwrap();

    let position = mock.query_position(&account_id);
    assert_eq!(position.coins.len(), 2);
    let osmo = get_coin("uosmo", &position.coins);
    assert_eq!(osmo.amount.u128(), 143);
    let atom = get_coin("uatom", &position.coins);
    assert_eq!(atom.amount.u128(), 57);
}
