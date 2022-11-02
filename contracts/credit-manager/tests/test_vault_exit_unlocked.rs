use cosmwasm_std::{Addr, Uint128};
use cw_utils::Duration;

use mock_vault::contract::STARTING_VAULT_SHARES;
use rover::adapters::vault::VaultUnchecked;
use rover::error::ContractError;
use rover::msg::execute::Action::{Deposit, EnterVault, ExitVaultUnlocked, RequestVaultUnlock};
use rover::msg::query::Positions;

use crate::helpers::{
    assert_err, generate_mock_vault, get_coin, locked_vault_info, lp_token_info, AccountToFund,
    MockEnv,
};

pub mod helpers;

#[test]
fn test_only_owner_can_withdraw_unlocked_for_account() {
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
        vec![ExitVaultUnlocked { id: 423, vault }],
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
        vec![ExitVaultUnlocked { id: 234, vault }],
        &[],
    );

    assert_err(res, ContractError::NotWhitelisted("xvault".to_string()));
}

#[test]
fn test_not_owner_of_unlocking_position() {
    let lp_token = lp_token_info();
    let leverage_vault = locked_vault_info();

    let user_a = Addr::unchecked("user_a");
    let user_b = Addr::unchecked("user_b");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: user_a.clone(),
            funds: vec![lp_token.to_coin(300)],
        })
        .fund_account(AccountToFund {
            addr: user_b.clone(),
            funds: vec![lp_token.to_coin(2)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let account_id_a = mock.create_credit_account(&user_a).unwrap();

    mock.update_credit_account(
        &account_id_a,
        &user_a,
        vec![
            Deposit(lp_token.to_coin(200)),
            EnterVault {
                vault: vault.clone(),
                coin: lp_token.to_coin(23),
            },
            RequestVaultUnlock {
                vault: vault.clone(),
                amount: STARTING_VAULT_SHARES,
            },
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    let positions = mock.query_positions(&account_id_a);
    assert_eq!(positions.vaults.len(), 1);
    let lockup_id = get_lockup_id(&positions);

    let account_id_b = mock.create_credit_account(&user_b).unwrap();

    let res = mock.update_credit_account(
        &account_id_b,
        &user_b,
        vec![
            Deposit(lp_token.to_coin(2)),
            EnterVault {
                vault: vault.clone(),
                coin: lp_token.to_coin(2),
            },
            RequestVaultUnlock {
                vault: vault.clone(),
                amount: STARTING_VAULT_SHARES,
            },
            ExitVaultUnlocked {
                id: lockup_id, // ID from user_a not from user_b
                vault,
            },
        ],
        &[lp_token.to_coin(2)],
    );

    assert_err(res, ContractError::NoPositionMatch(lockup_id.to_string()));
}

#[test]
fn test_unlocking_position_not_ready_time() {
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

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(200)),
            EnterVault {
                vault: vault.clone(),
                coin: lp_token.to_coin(23),
            },
            RequestVaultUnlock {
                vault: vault.clone(),
                amount: STARTING_VAULT_SHARES,
            },
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    let positions = mock.query_positions(&account_id);
    let lockup_id = get_lockup_id(&positions);

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![ExitVaultUnlocked {
            id: lockup_id,
            vault,
        }],
        &[],
    );

    assert_err(res, ContractError::UnlockNotReady);
}

#[test]
fn test_unlocking_position_not_ready_blocks() {
    let lp_token = lp_token_info();
    let leverage_vault = generate_mock_vault(Some(Duration::Height(100_000)));

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

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(200)),
            EnterVault {
                vault: vault.clone(),
                coin: lp_token.to_coin(23),
            },
            RequestVaultUnlock {
                vault: vault.clone(),
                amount: STARTING_VAULT_SHARES,
            },
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    let positions = mock.query_positions(&account_id);
    let lockup_id = get_lockup_id(&positions);

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![ExitVaultUnlocked {
            id: lockup_id,
            vault,
        }],
        &[],
    );

    assert_err(res, ContractError::UnlockNotReady);
}

#[test]
fn test_withdraw_unlock_success_time_expiring() {
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

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(200)),
            EnterVault {
                vault: vault.clone(),
                coin: lp_token.to_coin(200),
            },
            RequestVaultUnlock {
                vault: vault.clone(),
                amount: STARTING_VAULT_SHARES,
            },
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    let Positions { coins, .. } = mock.query_positions(&account_id);
    assert_eq!(coins.len(), 0);

    mock.app.update_block(|block| {
        if let Duration::Time(s) = leverage_vault.lockup.unwrap() {
            block.time = block.time.plus_seconds(s);
            block.height += 1;
        }
    });

    let positions = mock.query_positions(&account_id);
    let lockup_id = get_lockup_id(&positions);

    mock.update_credit_account(
        &account_id,
        &user,
        vec![ExitVaultUnlocked {
            id: lockup_id,
            vault,
        }],
        &[],
    )
    .unwrap();

    let Positions { vaults, coins, .. } = mock.query_positions(&account_id);

    // Users vault position decrements
    assert_eq!(vaults.len(), 0);

    // Users asset position increments
    let lp = get_coin(&lp_token.denom, &coins);
    assert_eq!(lp.amount, Uint128::from(200u128));

    // Assert Rover indeed has those on hand in the bank
    let lp = mock.query_balance(&mock.rover, &lp_token.denom);
    assert_eq!(lp.amount, Uint128::from(200u128));
}

#[test]
fn test_withdraw_unlock_success_block_expiring() {
    let lp_token = lp_token_info();
    let leverage_vault = generate_mock_vault(Some(Duration::Height(100_000)));

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

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(200)),
            EnterVault {
                vault: vault.clone(),
                coin: lp_token.to_coin(200),
            },
            RequestVaultUnlock {
                vault: vault.clone(),
                amount: STARTING_VAULT_SHARES,
            },
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    let Positions { coins, .. } = mock.query_positions(&account_id);
    assert_eq!(coins.len(), 0);

    mock.app.update_block(|block| {
        if let Duration::Height(h) = leverage_vault.lockup.unwrap() {
            block.height += h;
            block.time = block.time.plus_seconds(h * 6);
        }
    });

    let positions = mock.query_positions(&account_id);
    let lockup_id = get_lockup_id(&positions);

    mock.update_credit_account(
        &account_id,
        &user,
        vec![ExitVaultUnlocked {
            id: lockup_id,
            vault,
        }],
        &[],
    )
    .unwrap();

    let Positions { vaults, coins, .. } = mock.query_positions(&account_id);

    // Users vault position decrements
    assert_eq!(vaults.len(), 0);

    // Users asset position increments
    let lp = get_coin(&lp_token.denom, &coins);
    assert_eq!(lp.amount, Uint128::from(200u128));

    // Assert Rover indeed has those on hand in the bank
    let lp = mock.query_balance(&mock.rover, &lp_token.denom);
    assert_eq!(lp.amount, Uint128::from(200u128));
}

fn get_lockup_id(positions: &Positions) -> u64 {
    positions
        .vaults
        .first()
        .unwrap()
        .amount
        .unlocking()
        .positions()
        .first()
        .unwrap()
        .id
}
