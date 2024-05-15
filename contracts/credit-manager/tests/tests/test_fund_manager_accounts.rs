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

#[test]
fn addr_not_connected_to_fund_manager_acc_does_not_work() {
    let coin_info = uosmo_info();

    let random_addr = Addr::unchecked("random_addr");
    let fund_manager_wallet = Addr::unchecked("fund_manager_wallet");
    let fund_manager_vault = Addr::unchecked("fund_manager_vault");
    let funded_amt = Uint128::new(10000);
    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: random_addr.clone(),
            funds: coins(funded_amt.u128(), coin_info.denom.clone()),
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
    let fund_manager_vault = Addr::unchecked("fund_manager_vault");
    let funded_amt = Uint128::new(10000);
    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: fund_manager_wallet.clone(),
            funds: coins(funded_amt.u128(), coin_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: fund_manager_vault.clone(),
            funds: coins(funded_amt.u128(), coin_info.denom.clone()),
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

    // deposit from vault to fund manager account
    let deposit_amount = Uint128::new(234);
    mock.update_credit_account(
        &account_id,
        &fund_manager_vault,
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
    let coin = mock.query_balance(&fund_manager_vault, &coin_info.denom);
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
        &fund_manager_vault,
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
    let coin = mock.query_balance(&fund_manager_vault, &coin_info.denom);
    assert_eq!(coin.amount, funded_amt + Uint128::one()); // simulated yield
    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, Uint128::zero());
}
