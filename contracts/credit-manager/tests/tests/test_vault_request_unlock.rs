use cosmwasm_std::{coins, Addr, OverflowError, OverflowOperation::Sub, Uint128};
use cw_multi_test::{BankSudo, SudoMsg};
use cw_utils::{Duration, Expiration};
use mars_credit_manager::error::ContractError;
use mars_mock_vault::contract::STARTING_VAULT_SHARES;
use mars_types::{
    adapters::vault::{VaultError, VaultUnchecked},
    credit_manager::Action::{Deposit, EnterVault, RequestVaultUnlock},
};

use super::helpers::{
    assert_err, locked_vault_info, lp_token_info, unlocked_vault_info, AccountToFund, MockEnv,
};

#[test]
fn only_owner_can_request_unlocked() {
    let leverage_vault = locked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().vault_configs(&[leverage_vault.clone()]).build().unwrap();

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
fn can_only_take_action_on_whitelisted_vaults() {
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
fn request_when_unnecessary() {
    let leverage_vault = unlocked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().vault_configs(&[leverage_vault.clone()]).build().unwrap();

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
fn no_vault_tokens_for_request() {
    let leverage_vault = locked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().vault_configs(&[leverage_vault.clone()]).build().unwrap();

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
        VaultError::Overflow(OverflowError {
            operation: Sub,
            operand1: "0".to_string(),
            operand2: STARTING_VAULT_SHARES.to_string(),
        })
        .into(),
    );
}

#[test]
fn not_enough_vault_tokens_for_request() {
    let lp_token = lp_token_info();
    let leverage_vault = locked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone()])
        .vault_configs(&[leverage_vault.clone()])
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
                coin: lp_token.to_action_coin(23),
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
        VaultError::Overflow(OverflowError {
            operation: Sub,
            operand1: STARTING_VAULT_SHARES.to_string(),
            operand2: (STARTING_VAULT_SHARES + Uint128::new(100)).to_string(),
        })
        .into(),
    );
}

#[test]
fn request_unlocked() {
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

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(200)),
            EnterVault {
                vault: vault.clone(),
                coin: lp_token.to_action_coin(23),
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

#[test]
fn cannot_request_more_than_max() {
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
        .max_unlocking_positions(3)
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
                coin: lp_token.to_action_coin(23),
            },
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    // First three positions are allowed (at max)
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            RequestVaultUnlock {
                vault: vault.clone(),
                amount: Uint128::new(100),
            },
            RequestVaultUnlock {
                vault: vault.clone(),
                amount: Uint128::new(100),
            },
            RequestVaultUnlock {
                vault: vault.clone(),
                amount: Uint128::new(100),
            },
        ],
        &[],
    )
    .unwrap();

    // next one goes over max
    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![RequestVaultUnlock {
            vault,
            amount: Uint128::new(100),
        }],
        &[],
    );

    assert_err(
        res,
        ContractError::ExceedsMaxUnlockingPositions {
            new_amount: Uint128::new(4),
            maximum: Uint128::new(3),
        },
    )
}
