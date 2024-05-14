use cosmwasm_std::{coins, Addr, Coin, Uint128};
use mars_credit_manager::error::ContractError;
use mars_types::{
    credit_manager::{Action, ActionAmount, ActionCoin},
    health::AccountKind,
};

use super::helpers::{assert_err, uosmo_info, AccountToFund, MockEnv};

#[test]
fn fund_manager_wallet_cannot_deposit_and_withdraw() {
    let coin_info = uosmo_info();

    let fund_manager_wallet = Addr::unchecked("fund_manager_wallet");
    let fund_manager_vault = Addr::unchecked("fund_manager_vault");
    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: fund_manager_wallet.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock
        .create_credit_account_v2(
            &fund_manager_wallet,
            AccountKind::FundManager {
                vault_addr: fund_manager_vault.to_string(),
            },
            None,
        )
        .unwrap();

    // deposit not allowed
    let deposit_amount = Uint128::new(234);
    let res = mock.update_credit_account(
        &account_id,
        &fund_manager_wallet,
        vec![Action::Deposit(coin_info.to_coin(deposit_amount.u128()))],
        &[Coin::new(deposit_amount.into(), coin_info.denom.clone())],
    );
    assert_err(
        res,
        ContractError::Unauthorized {
            user: account_id.to_string(),
            action: "deposit, withdraw, refund_all_coin_balances".to_string(),
        },
    );

    // withdraw not allowed
    let res = mock.update_credit_account(
        &account_id,
        &fund_manager_wallet,
        vec![Action::Withdraw(ActionCoin {
            denom: coin_info.denom.clone(),
            amount: ActionAmount::AccountBalance,
        })],
        &[],
    );
    assert_err(
        res,
        ContractError::Unauthorized {
            user: account_id.to_string(),
            action: "deposit, withdraw, refund_all_coin_balances".to_string(),
        },
    );

    // refund_all_coin_balances not allowed
    let res = mock.update_credit_account(
        &account_id,
        &fund_manager_wallet,
        vec![Action::RefundAllCoinBalances {}],
        &[],
    );
    assert_err(
        res,
        ContractError::Unauthorized {
            user: account_id.to_string(),
            action: "deposit, withdraw, refund_all_coin_balances".to_string(),
        },
    );

    // combination of above msgs not allowed
    let res = mock.update_credit_account(
        &account_id,
        &fund_manager_wallet,
        vec![
            Action::Deposit(coin_info.to_coin(deposit_amount.u128())),
            Action::Withdraw(ActionCoin {
                denom: coin_info.denom.clone(),
                amount: ActionAmount::AccountBalance,
            }),
            Action::RefundAllCoinBalances {},
        ],
        &[Coin::new(deposit_amount.into(), coin_info.denom.clone())],
    );
    assert_err(
        res,
        ContractError::Unauthorized {
            user: account_id.to_string(),
            action: "deposit, withdraw, refund_all_coin_balances".to_string(),
        },
    );

    // not allowed action composed with allowed action
    let deposit_amount = Uint128::new(234);
    let res = mock.update_credit_account(
        &account_id,
        &fund_manager_wallet,
        vec![
            Action::Deposit(coin_info.to_coin(deposit_amount.u128())),
            Action::Lend(ActionCoin {
                denom: coin_info.denom.clone(),
                amount: ActionAmount::AccountBalance,
            }),
        ],
        &[Coin::new(deposit_amount.into(), coin_info.denom.clone())],
    );
    assert_err(
        res,
        ContractError::Unauthorized {
            user: account_id.to_string(),
            action: "deposit, withdraw, refund_all_coin_balances".to_string(),
        },
    );
}
