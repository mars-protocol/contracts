use cosmwasm_std::{coin, Addr, Uint128};
use mars_vault::{error::ContractError, msg::VaultUnlock};

use super::{
    helpers::{AccountToFund, MockEnv},
    vault_helpers::{
        assert_vault_err, execute_deposit, execute_unlock, query_total_vault_token_supply,
        query_user_unlocks,
    },
};
use crate::tests::{
    helpers::deploy_managed_vault,
    vault_helpers::{query_all_unlocks, query_convert_to_assets},
};

#[test]
fn unlock_if_credit_manager_account_not_binded() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);

    let res = execute_unlock(&mut mock, &user, &managed_vault_addr, Uint128::one(), &[]);
    assert_vault_err(res, ContractError::VaultAccountNotFound {});
}

#[test]
fn unlock_invalid_amount() {
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

    mock.create_fund_manager_account(&fund_manager, &managed_vault_addr);

    // unlock zero vault tokens
    let res = execute_unlock(&mut mock, &user, &managed_vault_addr, Uint128::zero(), &[]);
    assert_vault_err(
        res,
        ContractError::InvalidAmount {
            reason: "provided zero vault tokens".to_string(),
        },
    );

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

    // unlock more than total vault tokens supply
    let total_vault_supply = query_total_vault_token_supply(&mock, &managed_vault_addr);
    let res = execute_unlock(
        &mut mock,
        &user,
        &managed_vault_addr,
        total_vault_supply + Uint128::one(),
        &[],
    );
    assert_vault_err(
        res,
        ContractError::InvalidAmount {
            reason: "amount exceeds total vault token supply".to_string(),
        },
    );
}

#[test]
fn unlock_succeded() {
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

    mock.create_fund_manager_account(&fund_manager, &managed_vault_addr);

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

    let total_vault_supply = query_total_vault_token_supply(&mock, &managed_vault_addr);

    // first unlock
    let first_block_time = mock.query_block_time();
    let first_unlock_amt = total_vault_supply.multiply_ratio(1u128, 4u128); // unlock 1/4
    let first_base_tokens = query_convert_to_assets(&mock, &managed_vault_addr, first_unlock_amt);
    execute_unlock(&mut mock, &user, &managed_vault_addr, first_unlock_amt, &[]).unwrap();

    mock.increment_by_time(250); // 250 seconds

    // second unlock
    let second_block_time = mock.query_block_time();
    let second_unlock_amt = total_vault_supply.multiply_ratio(2u128, 4u128); // unlock 2/4
    let second_base_tokens = query_convert_to_assets(&mock, &managed_vault_addr, first_unlock_amt);
    execute_unlock(&mut mock, &user, &managed_vault_addr, second_unlock_amt, &[]).unwrap();

    let user_unlocks = query_user_unlocks(&mock, &managed_vault_addr, &user);
    assert_eq!(
        user_unlocks.clone(),
        vec![
            VaultUnlock {
                user_address: user.to_string(),
                created_at: first_block_time,
                cooldown_end: first_block_time + 60,
                vault_tokens: first_unlock_amt,
                base_tokens: first_base_tokens
            },
            VaultUnlock {
                user_address: user.to_string(),
                created_at: second_block_time,
                cooldown_end: second_block_time + 60,
                vault_tokens: second_unlock_amt,
                base_tokens: second_base_tokens
            }
        ]
    );

    let all_unlocks = query_all_unlocks(&mock, &managed_vault_addr, None, None);
    assert!(!all_unlocks.metadata.has_more);
    assert_eq!(user_unlocks.clone(), all_unlocks.data);

    let all_unlocks = query_all_unlocks(
        &mock,
        &managed_vault_addr,
        Some((user.to_string(), first_block_time)),
        None,
    );
    assert!(!all_unlocks.metadata.has_more);
    assert_eq!(all_unlocks.data.len(), 1);
    assert_eq!(user_unlocks[1..], all_unlocks.data); // only the second unlock
}
