use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use cw_utils::PaymentError;
use mars_vault::error::ContractError;

use super::{
    helpers::{AccountToFund, MockEnv},
    vault_helpers::{assert_vault_err, execute_deposit},
};
use crate::tests::{
    helpers::deploy_managed_vault,
    test_redeem::uusdc_info,
    vault_helpers::{query_total_assets, query_total_vault_token_supply, query_vault_info},
};

#[test]
fn deposit_invalid_funds() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(1_000_000_000, "untrn"), coin(1_000_000_000, "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);

    mock.create_fund_manager_account(&fund_manager, &managed_vault_addr);

    let res = execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[],
    );
    assert_vault_err(res, ContractError::Payment(PaymentError::NoFunds {}));

    let res = execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(1_001, "untrn"), coin(1_002, "uusdc")],
    );
    assert_vault_err(res, ContractError::Payment(PaymentError::MultipleDenoms {}));

    let res = execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(1_001, "untrn")],
    );
    assert_vault_err(res, ContractError::Payment(PaymentError::MissingDenom("uusdc".to_string())));
}

#[test]
fn deposit_if_credit_manager_account_not_binded() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(1_000_000_000, "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);

    let deposited_amt = Uint128::new(123_000_000);
    let res = execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    );
    assert_vault_err(res, ContractError::VaultAccountNotFound {});
}

#[test]
fn deposit_succeded() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let user_funded_amt = Uint128::new(1_000_000_000);
    let mut mock = MockEnv::new()
        .set_params(&[uusdc_info()])
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(user_funded_amt.u128(), "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);
    let vault_info_res = query_vault_info(&mock, &managed_vault_addr);
    let vault_token = vault_info_res.vault_token;

    // there shouldn't be any vault tokens
    let vault_token_balance = mock.query_balance(&managed_vault_addr, &vault_token).amount;
    assert!(vault_token_balance.is_zero());
    let vault_token_balance = mock.query_balance(&user, &vault_token).amount;
    assert!(vault_token_balance.is_zero());

    let account_id = mock.create_fund_manager_account(&fund_manager, &managed_vault_addr);

    let deposited_amt = Uint128::new(123_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    )
    .unwrap();

    // check base token balance after deposit
    let user_base_token_balance = mock.query_balance(&user, "uusdc").amount;
    assert_eq!(user_base_token_balance, user_funded_amt - deposited_amt);

    // there should be vault tokens for the user now
    let vault_token_balance = mock.query_balance(&managed_vault_addr, &vault_token).amount;
    assert!(vault_token_balance.is_zero());
    let user_vault_token_balance = mock.query_balance(&user, &vault_token).amount;
    assert!(!user_vault_token_balance.is_zero());
    assert_eq!(user_vault_token_balance, deposited_amt * Uint128::new(1_000_000));

    // there should be a deposit in Fund Manager's account
    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 1);
    let assets_res = res.deposits.first().unwrap();
    assert_eq!(assets_res.amount, deposited_amt);
    assert_eq!(assets_res.denom, "uusdc".to_string());

    // check total base/vault tokens and share price
    let vault_info_res = query_vault_info(&mock, &managed_vault_addr);
    let total_base_tokens = query_total_assets(&mock, &managed_vault_addr);
    let total_vault_tokens = query_total_vault_token_supply(&mock, &managed_vault_addr);
    assert_eq!(total_base_tokens, deposited_amt);
    assert_eq!(total_vault_tokens, user_vault_token_balance);
    assert_eq!(vault_info_res.total_base_tokens, total_base_tokens);
    assert_eq!(vault_info_res.total_vault_tokens, total_vault_tokens);
    assert_eq!(
        vault_info_res.share_price,
        Some(Decimal::from_ratio(total_base_tokens, total_vault_tokens))
    );
}
