use cosmwasm_std::{coin, coins, Addr, StdError, Uint128};
use mars_red_bank_types::oracle::ActionKind;
use mars_rover::{
    error::ContractError,
    msg::execute::{
        Action::{Borrow, Deposit, Repay, Withdraw},
        ActionAmount, ActionCoin,
    },
};

use crate::helpers::{assert_err, get_coin, get_debt, uosmo_info, AccountToFund, MockEnv};

pub mod helpers;

#[test]
fn deposit_and_repay_works_without_hf_check() {
    let coin_info = uosmo_info();

    let user = Addr::unchecked("user");

    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(1000, coin_info.denom.clone()),
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 0);
    assert_eq!(position.debts.len(), 0);

    // Create a debt in the account
    mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(coin_info.to_coin(300)), Borrow(coin_info.to_coin(50))],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(get_coin(&coin_info.denom, &position.deposits).amount, Uint128::new(350));
    assert_eq!(position.debts.len(), 1);
    assert_eq!(get_debt(&coin_info.denom, &position.debts).amount, Uint128::new(51)); // +1 simulated interest

    let coin_balance = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin_balance.amount, Uint128::new(350));

    // Simulate a problem with Default pricing. The price should be used for HF check before and after bunch of the actions,
    // but because of the price problem HF check is not possible.
    mock.remove_price(&coin_info.denom, ActionKind::Default);

    // Deposit should pass. HF check should be skipped
    mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(coin_info.to_coin(34))],
        &[coin(34, &coin_info.denom)],
    )
    .unwrap();

    // Repay part of the debt. HF check should be skipped
    mock.update_credit_account(
        &account_id,
        &user,
        vec![Repay {
            recipient_account_id: None,
            coin: coin_info.to_action_coin(20),
        }],
        &[],
    )
    .unwrap();

    // Deposit and repay in the same TX. HF check should be skipped
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(coin_info.to_coin(12)),
            Repay {
                recipient_account_id: None,
                coin: coin_info.to_action_coin(12),
            },
        ],
        &[coin(12, &coin_info.denom)],
    )
    .unwrap();

    // Repay for recepient should fail because of HF check
    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Repay {
            recipient_account_id: Some("random_account_id".to_string()),
            coin: coin_info.to_action_coin(20),
        }],
        &[],
    );
    assert_err(res, ContractError::Std(StdError::generic_err(
        "Querier contract error: Generic error: Querier contract error: cosmwasm_std::math::decimal::Decimal not found".to_string()
    )));

    // Deposit, repay and withdraw in the same TX. Should fail because of HF check
    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(coin_info.to_coin(12)),
            Repay {
                recipient_account_id: None,
                coin: coin_info.to_action_coin(12),
            },
            Withdraw(ActionCoin {
                denom: coin_info.denom.clone(),
                amount: ActionAmount::AccountBalance,
            }),
        ],
        &[coin(12, &coin_info.denom)],
    );
    assert_err(res, ContractError::Std(StdError::generic_err(
        "Querier contract error: Generic error: Querier contract error: cosmwasm_std::math::decimal::Decimal not found".to_string()
    )));

    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(get_coin(&coin_info.denom, &position.deposits).amount, Uint128::new(364));
    assert_eq!(position.debts.len(), 1);
    assert_eq!(get_debt(&coin_info.denom, &position.debts).amount, Uint128::new(19));

    let coin_balance = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin_balance.amount, Uint128::new(364));
}

#[test]
fn withdraw_works_without_hf_check_if_no_debt() {
    let coin_info = uosmo_info();

    let user = Addr::unchecked("user");

    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(1000, coin_info.denom.clone()),
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 0);
    assert_eq!(position.debts.len(), 0);

    // Create a debt in the account
    mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(coin_info.to_coin(300)), Borrow(coin_info.to_coin(50))],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(get_coin(&coin_info.denom, &position.deposits).amount, Uint128::new(350));
    assert_eq!(position.debts.len(), 1);
    assert_eq!(get_debt(&coin_info.denom, &position.debts).amount, Uint128::new(51)); // +1 simulated interest

    let coin_balance = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin_balance.amount, Uint128::new(350));

    // Simulate a problem with Default pricing. The price should be used for HF check before and after bunch of the actions,
    // but because of the price problem HF check is not possible.
    mock.remove_price(&coin_info.denom, ActionKind::Default);

    // Withdraw with existing debt in the account. Should fail because of HF check
    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Withdraw(ActionCoin {
            denom: coin_info.denom.clone(),
            amount: ActionAmount::AccountBalance,
        })],
        &[],
    );
    assert_err(res, ContractError::Std(StdError::generic_err(
        "Querier contract error: Generic error: Querier contract error: cosmwasm_std::math::decimal::Decimal not found".to_string()
    )));

    // Repay full debt. HF check should be skipped
    mock.update_credit_account(
        &account_id,
        &user,
        vec![Repay {
            recipient_account_id: None,
            coin: coin_info.to_action_coin_full_balance(),
        }],
        &[],
    )
    .unwrap();

    // Withdraw if no debt in the account. HF check should be skipped
    mock.update_credit_account(
        &account_id,
        &user,
        vec![Withdraw(ActionCoin {
            denom: coin_info.denom.clone(),
            amount: ActionAmount::AccountBalance,
        })],
        &[],
    )
    .unwrap();

    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 0);
    assert_eq!(position.debts.len(), 0);

    let coin_balance = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin_balance.amount, Uint128::zero());
}
