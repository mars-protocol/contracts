use std::ops::{Add, Sub};

use cosmwasm_std::{coin, coins, Addr, OverflowError, OverflowOperation, Uint128};
use mars_credit_manager::error::ContractError;
use mars_types::credit_manager::{
    Action::{Borrow, Deposit, Repay},
    ActionAmount, ActionCoin, CallbackMsg,
};

use super::helpers::{
    assert_err, uatom_info, uosmo_info, AccountToFund, MockEnv, DEFAULT_RED_BANK_COIN_BALANCE,
};
use crate::tests::helpers::{get_coin, get_debt};

#[test]
fn only_rover_can_call_repay_for_recipient_callback() {
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().build().unwrap();

    let res = mock.invoke_callback(
        &user,
        CallbackMsg::RepayForRecipient {
            benefactor_account_id: "abc".to_string(),
            recipient_account_id: "xyz".to_string(),
            coin: ActionCoin {
                denom: "udoge".to_string(),
                amount: ActionAmount::AccountBalance,
            },
        },
    );
    assert_err(res, ContractError::ExternalInvocation)
}

#[test]
fn raises_when_benefactor_has_no_funds() {
    let coin_info = uosmo_info();

    let recipient = Addr::unchecked("recipient");
    let benefactor = Addr::unchecked("benefactor");

    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: recipient.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();

    let recipient_account_id = mock.create_credit_account(&recipient).unwrap();
    let benefactor_account_id = mock.create_credit_account(&benefactor).unwrap();

    mock.update_credit_account(
        &recipient_account_id,
        &recipient,
        vec![Deposit(coin_info.to_coin(300)), Borrow(coin_info.to_coin(50))],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    let res = mock.update_credit_account(
        &benefactor_account_id,
        &benefactor,
        vec![
            Repay {
                recipient_account_id: Some(recipient_account_id.clone()),
                coin: coin_info.to_action_coin(51),
            }, // +1 for interest
        ],
        &[],
    );

    assert_err(
        res,
        ContractError::Overflow(OverflowError {
            operation: OverflowOperation::Sub,
            operand1: "0".to_string(),
            operand2: "51".to_string(),
        }),
    )
}

#[test]
fn raises_when_non_owner_of_benefactor_account_repays() {
    let coin_info = uosmo_info();

    let recipient = Addr::unchecked("recipient");
    let benefactor = Addr::unchecked("benefactor");

    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: benefactor.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: recipient.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();

    let recipient_account_id = mock.create_credit_account(&recipient).unwrap();
    let benefactor_account_id = mock.create_credit_account(&benefactor).unwrap();

    mock.update_credit_account(
        &recipient_account_id,
        &recipient,
        vec![Deposit(coin_info.to_coin(300)), Borrow(coin_info.to_coin(50))],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    mock.update_credit_account(
        &benefactor_account_id,
        &benefactor,
        vec![Deposit(coin_info.to_coin(300))],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    let res = mock.update_credit_account(
        &recipient_account_id,
        &benefactor,
        vec![
            Repay {
                recipient_account_id: Some(recipient_account_id.clone()),
                coin: coin_info.to_action_coin(51),
            }, // +1 for interest
        ],
        &[],
    );

    assert_err(
        res,
        ContractError::NotTokenOwner {
            user: benefactor.to_string(),
            account_id: recipient_account_id,
        },
    )
}

#[test]
fn raises_when_benefactor_repays_account_with_no_debt() {
    let coin_info = uosmo_info();

    let recipient = Addr::unchecked("recipient");
    let benefactor = Addr::unchecked("benefactor");

    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: benefactor.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: recipient.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();

    let recipient_account_id = mock.create_credit_account(&recipient).unwrap();
    let benefactor_account_id = mock.create_credit_account(&benefactor).unwrap();

    mock.update_credit_account(
        &recipient_account_id,
        &recipient,
        vec![Deposit(coin_info.to_coin(300))],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    mock.update_credit_account(
        &benefactor_account_id,
        &benefactor,
        vec![Deposit(coin_info.to_coin(300))],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    let res = mock.update_credit_account(
        &benefactor_account_id,
        &benefactor,
        vec![
            Repay {
                recipient_account_id: Some(recipient_account_id.clone()),
                coin: coin_info.to_action_coin(51),
            }, // +1 for interest
        ],
        &[],
    );

    assert_err(res, ContractError::NoDebt)
}

#[test]
fn benefactor_successfully_repays_on_behalf_of_recipient() {
    let coin_info = uosmo_info();

    let recipient = Addr::unchecked("recipient");
    let benefactor = Addr::unchecked("benefactor");

    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: benefactor.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: recipient.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();

    let recipient_account_id = mock.create_credit_account(&recipient).unwrap();
    let benefactor_account_id = mock.create_credit_account(&benefactor).unwrap();

    mock.update_credit_account(
        &recipient_account_id,
        &recipient,
        vec![Deposit(coin_info.to_coin(300)), Borrow(coin_info.to_coin(50))],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    mock.update_credit_account(
        &benefactor_account_id,
        &benefactor,
        vec![Deposit(coin_info.to_coin(300))],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    mock.update_credit_account(
        &benefactor_account_id,
        &benefactor,
        vec![
            Repay {
                recipient_account_id: Some(recipient_account_id.clone()),
                coin: coin_info.to_action_coin(51),
            }, // +1 for interest
        ],
        &[],
    )
    .unwrap();

    let recipient_position = mock.query_positions(&recipient_account_id.clone());
    assert_eq!(recipient_position.deposits.len(), 1);
    assert_eq!(recipient_position.deposits.first().unwrap().amount, Uint128::new(350));
    assert_eq!(recipient_position.debts.len(), 0);

    let benefactor_position = mock.query_positions(&benefactor_account_id.clone());
    assert_eq!(benefactor_position.deposits.len(), 1);
    assert_eq!(benefactor_position.deposits.first().unwrap().amount, Uint128::new(249));
    assert_eq!(benefactor_position.debts.len(), 0);

    let config = mock.query_config();
    let coin = mock.query_balance(&Addr::unchecked(config.red_bank), &coin_info.denom);
    assert_eq!(coin.amount, DEFAULT_RED_BANK_COIN_BALANCE.add(Uint128::new(1)));
}

#[test]
fn benefactor_pays_some_of_recipient_debt() {
    let coin_info = uosmo_info();

    let recipient = Addr::unchecked("recipient");
    let benefactor = Addr::unchecked("benefactor");

    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: benefactor.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: recipient.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();

    let recipient_account_id = mock.create_credit_account(&recipient).unwrap();
    let benefactor_account_id = mock.create_credit_account(&benefactor).unwrap();

    mock.update_credit_account(
        &recipient_account_id,
        &recipient,
        vec![Deposit(coin_info.to_coin(300)), Borrow(coin_info.to_coin(100))],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    mock.update_credit_account(
        &benefactor_account_id,
        &benefactor,
        vec![Deposit(coin_info.to_coin(50))],
        &[coin(50, coin_info.denom.clone())],
    )
    .unwrap();

    mock.update_credit_account(
        &benefactor_account_id,
        &benefactor,
        vec![Repay {
            recipient_account_id: Some(recipient_account_id.clone()),
            coin: coin_info.to_action_coin(50),
        }],
        &[],
    )
    .unwrap();

    let recipient_position = mock.query_positions(&recipient_account_id.clone());
    assert_eq!(recipient_position.deposits.len(), 1);
    assert_eq!(recipient_position.deposits.first().unwrap().amount, Uint128::new(400));
    assert_eq!(recipient_position.debts.len(), 1);
    assert_eq!(recipient_position.debts.first().unwrap().amount, Uint128::new(51));

    let benefactor_position = mock.query_positions(&benefactor_account_id.clone());
    assert_eq!(benefactor_position.deposits.len(), 0);
    assert_eq!(benefactor_position.debts.len(), 0);

    let config = mock.query_config();
    let coin = mock.query_balance(&Addr::unchecked(config.red_bank), &coin_info.denom);
    assert_eq!(coin.amount, DEFAULT_RED_BANK_COIN_BALANCE.sub(Uint128::new(50)));
    // total borrow = 100 - 50
}

#[test]
fn benefactor_attempts_to_pay_more_than_max_debt() {
    let coin_info = uosmo_info();

    let recipient = Addr::unchecked("recipient");
    let benefactor = Addr::unchecked("benefactor");

    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: benefactor.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: recipient.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();

    let recipient_account_id = mock.create_credit_account(&recipient).unwrap();
    let benefactor_account_id = mock.create_credit_account(&benefactor).unwrap();

    mock.update_credit_account(
        &recipient_account_id,
        &recipient,
        vec![Deposit(coin_info.to_coin(300)), Borrow(coin_info.to_coin(50))],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    mock.update_credit_account(
        &benefactor_account_id,
        &benefactor,
        vec![Deposit(coin_info.to_coin(300))],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    mock.update_credit_account(
        &benefactor_account_id,
        &benefactor,
        vec![
            Repay {
                recipient_account_id: Some(recipient_account_id.clone()),
                coin: coin_info.to_action_coin(110),
            }, // +1 for interest
        ],
        &[],
    )
    .unwrap();

    let recipient_position = mock.query_positions(&recipient_account_id.clone());
    assert_eq!(recipient_position.deposits.len(), 1);
    assert_eq!(recipient_position.deposits.first().unwrap().amount, Uint128::new(350));
    assert_eq!(recipient_position.debts.len(), 0);

    let benefactor_position = mock.query_positions(&benefactor_account_id.clone());
    assert_eq!(benefactor_position.deposits.len(), 1);
    assert_eq!(benefactor_position.deposits.first().unwrap().amount, Uint128::new(249));
    assert_eq!(benefactor_position.debts.len(), 0);

    let config = mock.query_config();
    let coin = mock.query_balance(&Addr::unchecked(config.red_bank), &coin_info.denom);
    assert_eq!(coin.amount, DEFAULT_RED_BANK_COIN_BALANCE.add(Uint128::new(1)));
}

#[test]
fn amount_none_repays_no_more_than_available_asset() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();

    let recipient = Addr::unchecked("recipient");
    let benefactor = Addr::unchecked("benefactor");

    let mut mock = MockEnv::new()
        .set_params(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: benefactor.clone(),
            funds: coins(300, uosmo_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: recipient.clone(),
            funds: coins(300, uatom_info.denom.clone()),
        })
        .build()
        .unwrap();

    let recipient_account_id = mock.create_credit_account(&recipient).unwrap();
    let benefactor_account_id = mock.create_credit_account(&benefactor).unwrap();

    mock.update_credit_account(
        &recipient_account_id,
        &recipient,
        vec![Deposit(uatom_info.to_coin(300)), Borrow(uosmo_info.to_coin(60))],
        &[coin(300, uatom_info.denom.clone())],
    )
    .unwrap();

    mock.update_credit_account(
        &benefactor_account_id,
        &benefactor,
        vec![
            Deposit(uosmo_info.to_coin(50)),
            Repay {
                recipient_account_id: Some(recipient_account_id.clone()),
                coin: uosmo_info.to_action_coin_full_balance(),
            },
        ],
        &[coin(50, uosmo_info.denom.clone())],
    )
    .unwrap();

    let recipient_position = mock.query_positions(&recipient_account_id.clone());
    assert_eq!(recipient_position.deposits.len(), 2);
    assert_eq!(get_coin(&uatom_info.denom, &recipient_position.deposits), uatom_info.to_coin(300));
    assert_eq!(get_coin(&uosmo_info.denom, &recipient_position.deposits), uosmo_info.to_coin(60));
    assert_eq!(recipient_position.debts.len(), 1);
    let uosmo_debt = get_debt(&uosmo_info.denom, &recipient_position.debts);
    // recipient debt: 60 uosmo,
    // benefactor account balance: 50 uosmo
    // repaying with benefactor full balance, should repay 50 uosmo
    assert_eq!(uosmo_debt.amount, Uint128::new(11)); // 10 + 1 interest

    let benefactor_position = mock.query_positions(&benefactor_account_id.clone());
    assert_eq!(benefactor_position.deposits.len(), 0);
    assert_eq!(benefactor_position.debts.len(), 0);
}
