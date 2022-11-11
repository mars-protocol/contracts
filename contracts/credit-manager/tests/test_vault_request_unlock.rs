use cosmwasm_std::OverflowOperation::Sub;
use cosmwasm_std::{coins, Addr, OverflowError, Uint128};
use cw_multi_test::{BankSudo, SudoMsg};
use cw_utils::{Duration, Expiration};

use mars_mock_vault::contract::STARTING_VAULT_SHARES;
use mars_rover::adapters::vault::VaultUnchecked;
use mars_rover::error::ContractError;
use mars_rover::msg::execute::Action::{Deposit, EnterVault, RequestVaultUnlock};

use crate::helpers::{
    assert_err, locked_vault_info, lp_token_info, unlocked_vault_info, AccountToFund, MockEnv,
};

pub mod helpers;

#[test]
fn test_only_owner_can_request_unlocked() {
    let leverage_vault = locked_vault_info();

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
        vec![RequestVaultUnlock {
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
        vec![RequestVaultUnlock {
            vault,
            amount: STARTING_VAULT_SHARES,
        }],
        &[],
    );

    assert_err(res, ContractError::NotWhitelisted("xvault".to_string()));
}

#[test]
fn test_request_when_unnecessary() {
    let leverage_vault = unlocked_vault_info();

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
        vec![RequestVaultUnlock {
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
fn test_no_vault_tokens_for_request() {
    let leverage_vault = locked_vault_info();

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
            amount: coins(5_000_000, leverage_vault.vault_token_denom),
        }))
        .unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![RequestVaultUnlock {
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
fn test_not_enough_vault_tokens_for_request() {
    let lp_token = lp_token_info();
    let leverage_vault = locked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![lp_token.to_coin(300)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let account_id = mock.create_credit_account(&user).unwrap();

    // Seed Rover with vault tokens
    mock.app
        .sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: mock.rover.clone().to_string(),
            amount: coins(5_000_000, leverage_vault.vault_token_denom),
        }))
        .unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(200)),
            EnterVault {
                vault: vault.clone(),
                denom: lp_token.denom.clone(),
                amount: Some(Uint128::new(23)),
            },
            RequestVaultUnlock {
                vault,
                amount: STARTING_VAULT_SHARES + Uint128::new(100),
            },
        ],
        &[lp_token.to_coin(200)],
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
    let lp_token = lp_token_info();
    let leverage_vault = locked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![lp_token.to_coin(200)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let account_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(200)),
            EnterVault {
                vault: vault.clone(),
                denom: lp_token.denom.clone(),
                amount: Some(Uint128::new(23)),
            },
            RequestVaultUnlock {
                vault: vault.clone(),
                amount: STARTING_VAULT_SHARES,
            },
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    // Assert token's position with Rover
    let res = mock.query_positions(&account_id);
    assert_eq!(res.vaults.len(), 1);
    let unlocking = res.vaults.first().unwrap().amount.unlocking();
    assert_eq!(unlocking.positions().len(), 1);
    let positions = unlocking.positions();
    let first = positions.first().unwrap();
    assert_eq!(first.coin.amount, Uint128::new(23));

    match leverage_vault.lockup.unwrap() {
        Duration::Height(_) => panic!("wrong type of duration"),
        Duration::Time(s) => {
            let expected_unlock_time = mock.app.block_info().time.seconds() + s;
            let unlocking_position = mock.query_unlocking_position(&vault, first.id);

            match unlocking_position.release_at {
                Expiration::AtTime(t) => {
                    assert_eq!(t.seconds(), expected_unlock_time);
                }
                _ => panic!("Wrong type of expiration"),
            }

            // Assert Rover's position w/ Vault
            let res = mock.query_unlocking_positions(&vault, &mock.rover);

            match res.first().unwrap().release_at {
                Expiration::AtTime(t) => {
                    assert_eq!(res.len(), 1);
                    assert_eq!(res.first().unwrap().base_token_amount, Uint128::new(23));
                    assert_eq!(t.seconds(), expected_unlock_time);
                }
                _ => panic!("Wrong type of expiration"),
            }
        }
    }
}
