use cosmwasm_std::{coins, Addr, Coin, Empty, Uint128};
use cw721::OwnerOfResponse;
use cw721_base::QueryMsg as NftQueryMsg;
use mars_types::{credit_manager::Action, health::AccountKind};

use super::helpers::MockEnv;
use crate::tests::helpers::{uosmo_info, AccountToFund};

#[test]
fn update_credit_account_fails_without_nft_contract_set() {
    let mut mock = MockEnv::new().no_nft_contract().build().unwrap();
    let user = Addr::unchecked("user");
    let res = mock.update_credit_account_with_new_acc(None, None, &user, vec![], &[]);

    if res.is_ok() {
        panic!("Should have thrown error due to nft contract not yet set");
    }
}

#[test]
fn update_credit_account_fails_without_nft_contract_owner() {
    let mut mock = MockEnv::new().no_nft_contract_minter().build().unwrap();

    let user = Addr::unchecked("user");
    let res = mock.update_credit_account_with_new_acc(None, None, &user, vec![], &[]);

    if res.is_ok() {
        panic!("Should have thrown error due to nft contract not proposing a new owner yet");
    }
}

#[test]
fn update_credit_account_with_new_default_account_created() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user_123");
    let response = mock
        .update_credit_account_with_new_acc(None, Some(AccountKind::Default), &user, vec![], &[])
        .unwrap();
    let account_id = mock.get_account_id(response);

    // Double checking ownership by querying NFT account-nft for correct owner
    let config = mock.query_config();

    let owner_res: OwnerOfResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            config.account_nft.unwrap(),
            &NftQueryMsg::<Empty>::OwnerOf {
                token_id: account_id.clone(),
                include_expired: None,
            },
        )
        .unwrap();
    assert_eq!(user, owner_res.owner);

    let acc_kind = mock.query_account_kind(&account_id);
    assert_eq!(acc_kind, AccountKind::Default);
}

#[test]
fn update_credit_account_with_new_hls_account_created() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user_123");
    let response = mock
        .update_credit_account_with_new_acc(
            None,
            Some(AccountKind::HighLeveredStrategy),
            &user,
            vec![],
            &[],
        )
        .unwrap();
    let account_id = mock.get_account_id(response);

    // Double checking ownership by querying NFT account-nft for correct owner
    let config = mock.query_config();

    let owner_res: OwnerOfResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            config.account_nft.unwrap(),
            &NftQueryMsg::<Empty>::OwnerOf {
                token_id: account_id.clone(),
                include_expired: None,
            },
        )
        .unwrap();
    assert_eq!(user, owner_res.owner);

    let acc_kind = mock.query_account_kind(&account_id);
    assert_eq!(acc_kind, AccountKind::HighLeveredStrategy);
}

#[test]
fn update_credit_account_with_existing_account_and_extra_kind_provided() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user_123");
    let account_id = mock.create_credit_account(&user).unwrap();

    let user_accs = mock.query_accounts(user.as_str(), None, None);
    assert_eq!(user_accs.len(), 1);

    let acc_kind = mock.query_account_kind(&account_id);
    assert_eq!(acc_kind, AccountKind::Default);

    // update the account with a new kind
    let _response = mock
        .update_credit_account_with_new_acc(
            Some(account_id.clone()),
            Some(AccountKind::HighLeveredStrategy),
            &user,
            vec![],
            &[],
        )
        .unwrap();

    // should have only one account
    let user_accs = mock.query_accounts(user.as_str(), None, None);
    assert_eq!(user_accs.len(), 1);

    // account kind shouldn't be changed
    let acc_kind = mock.query_account_kind(&account_id);
    assert_eq!(acc_kind, AccountKind::Default);
}

#[test]
fn deposit_straight_to_new_account() {
    let coin_info = uosmo_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();

    let deposit_amount = Uint128::new(234);
    let response = mock
        .update_credit_account_with_new_acc(
            None,
            None,
            &user,
            vec![Action::Deposit(coin_info.to_coin(deposit_amount.u128()))],
            &[Coin::new(deposit_amount.into(), coin_info.denom.clone())],
        )
        .unwrap();
    let account_id = mock.get_account_id(response);

    let res = mock.query_positions(&account_id);
    let assets_res = res.deposits.first().unwrap();
    assert_eq!(res.deposits.len(), 1);
    assert_eq!(assets_res.amount, deposit_amount);
    assert_eq!(assets_res.denom, coin_info.denom);

    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, deposit_amount)
}
