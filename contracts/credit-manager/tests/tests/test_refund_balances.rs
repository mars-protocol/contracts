use cosmwasm_std::{coin, Addr, Uint128};
use mars_types::credit_manager::Action::{Deposit, EnterVault, RefundAllCoinBalances};

use super::helpers::{
    locked_vault_info, lp_token_info, uatom_info, uosmo_info, AccountToFund, MockEnv,
};

#[test]
fn refund_coin_balances_when_balances() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(234, uosmo_info.denom.clone()), coin(25, uatom_info.denom.clone())],
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(uosmo_info.to_coin(234)),
            Deposit(uatom_info.to_coin(25)),
            RefundAllCoinBalances {},
        ],
        &[uosmo_info.to_coin(234), uatom_info.to_coin(25)],
    )
    .unwrap();

    // Assert refunds have been issued
    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);

    let osmo_balance = mock.query_balance(&user, &uosmo_info.denom);
    assert_eq!(osmo_balance.amount, Uint128::new(234));
    let atom_balance = mock.query_balance(&user, &uatom_info.denom);
    assert_eq!(atom_balance.amount, Uint128::new(25));
}

#[test]
fn refund_coin_balances_when_no_balances() {
    let lp_token = lp_token_info();
    let leverage_vault = locked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone()])
        .vault_configs(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![lp_token.to_coin(200)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let account_id = mock.create_credit_account(&user).unwrap();
    let balance = mock.query_total_vault_coin_balance(&vault);
    assert_eq!(balance, Uint128::zero());

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(200)),
            EnterVault {
                vault,
                coin: lp_token.to_action_coin(200),
            },
            RefundAllCoinBalances {},
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    // Assert no error is thrown and nothing happens to coin balances
    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);
    // Assert vault positions have not been effected
    assert_eq!(res.vaults.len(), 1);

    // Assert nothing has been refunded to wallet
    let lp_balance = mock.query_balance(&user, &lp_token.denom);
    assert_eq!(lp_balance.amount, Uint128::zero());
}
