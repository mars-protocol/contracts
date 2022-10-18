use cosmwasm_std::StdError::NotFound;
use cosmwasm_std::{coin, Addr, Uint128};

use mock_vault::contract::STARTING_VAULT_SHARES;
use rover::adapters::VaultUnchecked;
use rover::error::ContractError;
use rover::msg::execute::Action::{
    Deposit, VaultDeposit, VaultRequestUnlock, VaultWithdrawUnlocked,
};
use rover::msg::query::Positions;

use crate::helpers::{
    assert_err, get_coin, uatom_info, uosmo_info, AccountToFund, MockEnv, VaultTestInfo,
};

pub mod helpers;

#[test]
fn test_only_owner_can_withdraw_unlocked_for_account() {
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
        vec![VaultWithdrawUnlocked { id: 423, vault }],
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
        vec![VaultWithdrawUnlocked { id: 234, vault }],
        &[],
    );

    assert_err(res, ContractError::NotWhitelisted("xvault".to_string()));
}

#[test]
fn test_not_owner_of_unlocking_position() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();

    let leverage_vault = VaultTestInfo {
        denom: "uleverage".to_string(),
        lockup: Some(1_209_600), // 14 days
        underlying_denoms: vec!["uatom".to_string(), "uosmo".to_string()],
    };

    let user_a = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uatom.clone(), uosmo.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: user_a.clone(),
            funds: vec![coin(300, "uatom"), coin(500, "uosmo")],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let account_id_a = mock.create_credit_account(&user_a).unwrap();

    mock.update_credit_account(
        &account_id_a,
        &user_a,
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

    let res = mock.query_positions(&account_id_a);
    assert_eq!(res.vaults.len(), 1);
    let unlocking_id = res
        .vaults
        .first()
        .unwrap()
        .state
        .unlocking
        .first()
        .unwrap()
        .id;

    let user_b = Addr::unchecked("user_b");
    let account_id_b = mock.create_credit_account(&user_b).unwrap();

    let res = mock.update_credit_account(
        &account_id_b,
        &user_b,
        vec![VaultWithdrawUnlocked {
            id: unlocking_id,
            vault,
        }],
        &[],
    );

    assert_err(
        res,
        ContractError::Std(NotFound {
            kind: "rover::adapters::vault::VaultPositionState".to_string(),
        }),
    );
}

#[test]
fn test_unlocking_position_not_ready() {
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

    let Positions { vaults, .. } = mock.query_positions(&account_id);

    let position_id = vaults.first().unwrap().state.unlocking.first().unwrap().id;

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![VaultWithdrawUnlocked {
            id: position_id,
            vault,
        }],
        &[],
    );

    assert_err(res, ContractError::UnlockNotReady);
}

#[test]
fn test_withdraw_unlock_success() {
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
                coins: vec![coin(200, "uatom"), coin(400, "uosmo")],
            },
            VaultRequestUnlock {
                vault: vault.clone(),
                amount: STARTING_VAULT_SHARES,
            },
        ],
        &[coin(200, "uatom"), coin(400, "uosmo")],
    )
    .unwrap();

    let Positions { coins, .. } = mock.query_positions(&account_id);
    assert_eq!(coins.len(), 0);

    mock.app.update_block(|block| {
        block.time = block.time.plus_seconds(leverage_vault.lockup.unwrap());
        block.height += 1;
    });

    let Positions { vaults, .. } = mock.query_positions(&account_id);

    let position_id = vaults.first().unwrap().state.unlocking.first().unwrap().id;

    mock.update_credit_account(
        &account_id,
        &user,
        vec![VaultWithdrawUnlocked {
            id: position_id,
            vault,
        }],
        &[],
    )
    .unwrap();

    let Positions { vaults, coins, .. } = mock.query_positions(&account_id);

    // Users vault position decrements
    assert_eq!(vaults.len(), 0);

    // Users asset position increments
    let atom = get_coin("uatom", &coins);
    assert_eq!(atom.amount, Uint128::from(200u128));
    let osmo = get_coin("uosmo", &coins);
    assert_eq!(osmo.amount, Uint128::from(400u128));

    // Assert Rover indeed has those on hand in the bank
    let atom = mock.query_balance(&mock.rover, "uatom");
    assert_eq!(atom.amount, Uint128::from(200u128));
    let osmo = mock.query_balance(&mock.rover, "uosmo");
    assert_eq!(osmo.amount, Uint128::from(400u128));
}
