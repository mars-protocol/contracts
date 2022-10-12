use cosmwasm_std::OverflowOperation::Sub;
use cosmwasm_std::{coin, coins, Addr, OverflowError, Uint128};
use cw_multi_test::{BankSudo, SudoMsg};

use mock_vault::contract::STARTING_VAULT_SHARES;
use rover::adapters::VaultUnchecked;
use rover::error::ContractError;
use rover::msg::execute::Action::{Deposit, VaultDeposit, VaultRequestUnlock};

use crate::helpers::{assert_err, uatom_info, uosmo_info, AccountToFund, MockEnv, VaultTestInfo};

pub mod helpers;

#[test]
fn test_only_owner_can_request_unlocked() {
    let leverage_vault = VaultTestInfo {
        denom: "uleverage".to_string(),
        lockup: Some(1_209_600), // 14 days
        underlying_denoms: vec!["uatom".to_string(), "uosmo".to_string()],
    };

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_vaults(&[leverage_vault.clone()])
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let account_id = mock.create_credit_account(&user).unwrap();

    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.update_credit_account(
        &account_id,
        &bad_guy,
        vec![VaultRequestUnlock {
            vault,
            amount: STARTING_VAULT_SHARES,
        }],
        &[],
    );

    assert_err(
        res,
        ContractError::NotTokenOwner {
            user: bad_guy.to_string(),
            account_id,
        },
    );
}

#[test]
fn test_can_only_take_action_on_whitelisted_vaults() {
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().build().unwrap();

    let vault = VaultUnchecked::new("xvault".to_string());
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![VaultRequestUnlock {
            vault,
            amount: STARTING_VAULT_SHARES,
        }],
        &[],
    );

    assert_err(res, ContractError::NotWhitelisted("xvault".to_string()));
}

#[test]
fn test_request_when_unnecessary() {
    let leverage_vault = VaultTestInfo {
        denom: "uleverage".to_string(),
        lockup: None,
        underlying_denoms: vec!["uatom".to_string(), "uosmo".to_string()],
    };

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_vaults(&[leverage_vault.clone()])
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![VaultRequestUnlock {
            vault,
            amount: STARTING_VAULT_SHARES,
        }],
        &[],
    );

    assert_err(
        res,
        ContractError::RequirementsNotMet(
            "This vault does not require lockup. Call withdraw directly.".to_string(),
        ),
    );
}

#[test]
fn test_no_funds_for_request() {
    let leverage_vault = VaultTestInfo {
        denom: "uleverage".to_string(),
        lockup: Some(1_209_600), // 14 days
        underlying_denoms: vec!["uatom".to_string(), "uosmo".to_string()],
    };

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_vaults(&[leverage_vault.clone()])
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let account_id = mock.create_credit_account(&user).unwrap();

    // Seed Rover with vault tokens
    mock.app
        .sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: mock.rover.clone().to_string(),
            amount: coins(5_000_000, leverage_vault.denom),
        }))
        .unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![VaultRequestUnlock {
            vault,
            amount: STARTING_VAULT_SHARES,
        }],
        &[],
    );

    assert_err(
        res,
        ContractError::Overflow(OverflowError {
            operation: Sub,
            operand1: "0".to_string(),
            operand2: STARTING_VAULT_SHARES.to_string(),
        }),
    );
}

#[test]
fn test_not_enough_funds_for_request() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();

    let leverage_vault = VaultTestInfo {
        denom: "uleverage".to_string(),
        lockup: Some(1_209_600), // 14 days
        underlying_denoms: vec!["uatom".to_string(), "uosmo".to_string()],
    };

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uatom.clone(), uosmo.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(300, "uatom"), coin(500, "uosmo")],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let account_id = mock.create_credit_account(&user).unwrap();

    // Seed Rover with vault tokens
    mock.app
        .sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: mock.rover.clone().to_string(),
            amount: coins(5_000_000, leverage_vault.denom),
        }))
        .unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(coin(200, uatom.denom)),
            Deposit(coin(400, uosmo.denom)),
            VaultDeposit {
                vault: vault.clone(),
                coins: vec![coin(23, "uatom"), coin(120, "uosmo")],
            },
            VaultRequestUnlock {
                vault,
                amount: STARTING_VAULT_SHARES + Uint128::new(100),
            },
        ],
        &[coin(200, "uatom"), coin(400, "uosmo")],
    );

    assert_err(
        res,
        ContractError::Overflow(OverflowError {
            operation: Sub,
            operand1: STARTING_VAULT_SHARES.to_string(),
            operand2: (STARTING_VAULT_SHARES + Uint128::new(100)).to_string(),
        }),
    );
}

#[test]
fn test_request_unlocked() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();

    let leverage_vault = VaultTestInfo {
        denom: "uleverage".to_string(),
        lockup: Some(1_209_600), // 14 days
        underlying_denoms: vec!["uatom".to_string(), "uosmo".to_string()],
    };

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uatom.clone(), uosmo.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(300, "uatom"), coin(500, "uosmo")],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let account_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(coin(200, uatom.denom)),
            Deposit(coin(400, uosmo.denom)),
            VaultDeposit {
                vault: vault.clone(),
                coins: vec![coin(23, "uatom"), coin(120, "uosmo")],
            },
            VaultRequestUnlock {
                vault: vault.clone(),
                amount: STARTING_VAULT_SHARES,
            },
        ],
        &[coin(200, "uatom"), coin(400, "uosmo")],
    )
    .unwrap();

    // Assert token's position with Rover
    let res = mock.query_positions(&account_id);
    assert_eq!(res.vaults.len(), 1);
    let unlocking = res.vaults.first().unwrap().state.unlocking.clone();
    assert_eq!(unlocking.len(), 1);
    let first = unlocking.first().unwrap();
    assert_eq!(first.amount, STARTING_VAULT_SHARES);
    let expected_unlock_time =
        mock.app.block_info().time.seconds() + leverage_vault.lockup.unwrap();
    let unlocking_position = mock.query_unlocking_position_info(&vault, first.id);
    assert_eq!(
        unlocking_position.unlocked_at.seconds(),
        expected_unlock_time
    );

    // Assert Rover's position w/ Vault
    let res = mock.query_unlocking_positions(&vault, &mock.rover);
    assert_eq!(res.len(), 1);
    assert_eq!(res.first().unwrap().amount, STARTING_VAULT_SHARES);
    assert_eq!(
        res.first().unwrap().unlocked_at.seconds(),
        expected_unlock_time
    );
}
