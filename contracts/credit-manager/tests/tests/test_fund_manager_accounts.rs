use cosmwasm_std::{coin, coins, Addr, Coin, Uint128};
use cw_multi_test::{BankSudo, SudoMsg};
use cw_paginate::{Metadata, PaginationResponse};
use mars_credit_manager::error::ContractError;
use mars_types::{
    credit_manager::{Action, ActionAmount, ActionCoin, VaultBinding},
    health::AccountKind,
};

use super::helpers::{assert_err, deploy_managed_vault, uosmo_info, AccountToFund, MockEnv};

#[test]
fn fund_manager_wallet_cannot_deposit_and_withdraw() {
    let coin_info = uosmo_info();

    let fund_manager_wallet = Addr::unchecked("fund_manager_wallet");
    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: fund_manager_wallet.clone(),
            funds: vec![coin(1_000_000_000, "untrn"), coin(300, coin_info.denom.clone())],
        })
        .build()
        .unwrap();

    let credit_manager = mock.rover.clone();
    let managed_vault_addr =
        deploy_managed_vault(&mut mock.app, &fund_manager_wallet, &credit_manager);

    let account_id = mock
        .create_credit_account_v2(
            &fund_manager_wallet,
            AccountKind::FundManager {
                vault_addr: managed_vault_addr.to_string(),
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
            action: "deposit, withdraw, refund_all_coin_balances, withdraw_to_wallet".to_string(),
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
            action: "deposit, withdraw, refund_all_coin_balances, withdraw_to_wallet".to_string(),
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
            action: "deposit, withdraw, refund_all_coin_balances, withdraw_to_wallet".to_string(),
        },
    );

    // withdraw_to_wallet not allowed
    let res = mock.update_credit_account(
        &account_id,
        &fund_manager_wallet,
        vec![Action::WithdrawToWallet {
            coin: ActionCoin {
                denom: coin_info.denom.clone(),
                amount: ActionAmount::AccountBalance,
            },
            recipient: "wallet".to_string(),
        }],
        &[],
    );
    assert_err(
        res,
        ContractError::Unauthorized {
            user: account_id.to_string(),
            action: "deposit, withdraw, refund_all_coin_balances, withdraw_to_wallet".to_string(),
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
            Action::WithdrawToWallet {
                coin: ActionCoin {
                    denom: coin_info.denom.clone(),
                    amount: ActionAmount::AccountBalance,
                },
                recipient: "wallet".to_string(),
            },
        ],
        &[Coin::new(deposit_amount.into(), coin_info.denom.clone())],
    );
    assert_err(
        res,
        ContractError::Unauthorized {
            user: account_id.to_string(),
            action: "deposit, withdraw, refund_all_coin_balances, withdraw_to_wallet".to_string(),
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
            action: "deposit, withdraw, refund_all_coin_balances, withdraw_to_wallet".to_string(),
        },
    );
}

#[test]
fn addr_not_connected_to_fund_manager_acc_does_not_work() {
    let coin_info = uosmo_info();

    let random_addr = Addr::unchecked("random_addr");
    let fund_manager_wallet = Addr::unchecked("fund_manager_wallet");
    let funded_amt = Uint128::new(10000);
    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: random_addr.clone(),
            funds: vec![
                coin(1_000_000_000, "untrn"),
                coin(funded_amt.u128(), coin_info.denom.clone()),
            ],
        })
        .build()
        .unwrap();

    let credit_manager = mock.rover.clone();
    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &random_addr, &credit_manager);

    let account_id = mock
        .create_credit_account_v2(
            &fund_manager_wallet,
            AccountKind::FundManager {
                vault_addr: managed_vault_addr.to_string(),
            },
            None,
        )
        .unwrap();

    // try to deposit from different addr
    let deposit_amount = Uint128::new(234);
    let res = mock.update_credit_account(
        &account_id,
        &random_addr,
        vec![Action::Deposit(coin_info.to_coin(deposit_amount.u128()))],
        &[Coin::new(deposit_amount.into(), coin_info.denom.clone())],
    );
    assert_err(
        res,
        ContractError::NotTokenOwner {
            user: random_addr.to_string(),
            account_id: account_id.to_string(),
        },
    );
}

#[test]
fn fund_manager_wallet_can_work_on_behalf_of_vault() {
    let coin_info = uosmo_info();

    let fund_manager_wallet = Addr::unchecked("fund_manager_wallet");
    let funded_amt = Uint128::new(10000);
    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: fund_manager_wallet.clone(),
            funds: vec![
                coin(1_000_000_000, "untrn"),
                coin(funded_amt.u128(), coin_info.denom.clone()),
            ],
        })
        .build()
        .unwrap();

    let credit_manager = mock.rover.clone();
    let managed_vault_addr =
        deploy_managed_vault(&mut mock.app, &fund_manager_wallet, &credit_manager);
    mock.app
        .sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: managed_vault_addr.to_string(),
            amount: coins(funded_amt.u128(), coin_info.denom.clone()),
        }))
        .unwrap();

    let account_id = mock
        .create_credit_account_v2(
            &fund_manager_wallet,
            AccountKind::FundManager {
                vault_addr: managed_vault_addr.to_string(),
            },
            None,
        )
        .unwrap();

    // deposit from vault to fund manager account
    let deposit_amount = Uint128::new(234);
    mock.update_credit_account(
        &account_id,
        &managed_vault_addr,
        vec![Action::Deposit(coin_info.to_coin(deposit_amount.u128()))],
        &[Coin::new(deposit_amount.into(), coin_info.denom.clone())],
    )
    .unwrap();

    let res = mock.query_positions(&account_id);
    let assets_res = res.deposits.first().unwrap();
    assert_eq!(res.deposits.len(), 1);
    assert_eq!(assets_res.amount, deposit_amount);
    assert_eq!(assets_res.denom, coin_info.denom);

    let coin = mock.query_balance(&fund_manager_wallet, &coin_info.denom);
    assert_eq!(coin.amount, funded_amt);
    let coin = mock.query_balance(&managed_vault_addr, &coin_info.denom);
    assert_eq!(coin.amount, funded_amt - deposit_amount);
    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, deposit_amount);

    // execute lend from fund manager wallet
    mock.update_credit_account(
        &account_id,
        &fund_manager_wallet,
        vec![Action::Lend(ActionCoin {
            denom: coin_info.denom.clone(),
            amount: ActionAmount::AccountBalance,
        })],
        &[],
    )
    .unwrap();

    let res = mock.query_positions(&account_id);
    let lent_res = res.lends.first().unwrap();
    assert_eq!(res.lends.len(), 1);
    assert_eq!(lent_res.denom, coin_info.denom);
    let lent_amount = deposit_amount + Uint128::one(); // simulated yield
    assert_eq!(lent_res.amount, lent_amount);

    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, Uint128::zero());

    // vault unlend and withdraw
    mock.update_credit_account(
        &account_id,
        &managed_vault_addr,
        vec![
            Action::Reclaim(ActionCoin {
                denom: coin_info.denom.clone(),
                amount: ActionAmount::AccountBalance,
            }),
            Action::Withdraw(ActionCoin {
                denom: coin_info.denom.clone(),
                amount: ActionAmount::AccountBalance,
            }),
        ],
        &[],
    )
    .unwrap();

    let res = mock.query_positions(&account_id);
    assert!(res.deposits.is_empty());
    assert!(res.lends.is_empty());

    let coin = mock.query_balance(&fund_manager_wallet, &coin_info.denom);
    assert_eq!(coin.amount, funded_amt);
    let coin = mock.query_balance(&managed_vault_addr, &coin_info.denom);
    assert_eq!(coin.amount, funded_amt + Uint128::one()); // simulated yield
    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, Uint128::zero());
}

#[test]
fn vault_bindings() {
    let fund_manager_wallet = Addr::unchecked("fund_manager_wallet");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager_wallet.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .build()
        .unwrap();

    let credit_manager = mock.rover.clone();

    let vault_addr_1 = deploy_managed_vault(&mut mock.app, &fund_manager_wallet, &credit_manager);
    let fund_acc_id_1 = mock
        .create_credit_account_v2(
            &fund_manager_wallet,
            AccountKind::FundManager {
                vault_addr: vault_addr_1.to_string(),
            },
            None,
        )
        .unwrap();

    let res = mock.query_vault_bindings(None, None).unwrap();
    assert_eq!(
        res,
        PaginationResponse {
            data: vec![VaultBinding {
                account_id: fund_acc_id_1.clone(),
                vault_address: vault_addr_1.to_string()
            }],
            metadata: Metadata {
                has_more: false
            }
        }
    );

    let vault_addr_2 = deploy_managed_vault(&mut mock.app, &fund_manager_wallet, &credit_manager);
    let fund_acc_id_2 = mock
        .create_credit_account_v2(
            &fund_manager_wallet,
            AccountKind::FundManager {
                vault_addr: vault_addr_2.to_string(),
            },
            None,
        )
        .unwrap();
    let vault_addr_3 = deploy_managed_vault(&mut mock.app, &fund_manager_wallet, &credit_manager);
    let fund_acc_id_3 = mock
        .create_credit_account_v2(
            &fund_manager_wallet,
            AccountKind::FundManager {
                vault_addr: vault_addr_3.to_string(),
            },
            None,
        )
        .unwrap();

    let res = mock.query_vault_bindings(None, None).unwrap();
    assert_eq!(
        res,
        PaginationResponse {
            data: vec![
                VaultBinding {
                    account_id: fund_acc_id_1.clone(),
                    vault_address: vault_addr_1.to_string()
                },
                VaultBinding {
                    account_id: fund_acc_id_2.clone(),
                    vault_address: vault_addr_2.to_string()
                },
                VaultBinding {
                    account_id: fund_acc_id_3,
                    vault_address: vault_addr_3.to_string()
                }
            ],
            metadata: Metadata {
                has_more: false
            }
        }
    );

    let res = mock.query_vault_bindings(Some(fund_acc_id_1), Some(1)).unwrap();
    assert_eq!(
        res,
        PaginationResponse {
            data: vec![VaultBinding {
                account_id: fund_acc_id_2,
                vault_address: vault_addr_2.to_string()
            }],
            metadata: Metadata {
                has_more: true
            }
        }
    );
}
