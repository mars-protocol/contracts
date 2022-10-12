use cosmwasm_std::OverflowOperation::Sub;
use cosmwasm_std::{coin, Addr, Coin, OverflowError, Uint128};

use mock_vault::contract::STARTING_VAULT_SHARES;
use rover::adapters::VaultBase;
use rover::error::ContractError;
use rover::error::ContractError::{NotTokenOwner, NotWhitelisted};
use rover::msg::execute::Action::{Deposit, VaultDeposit, VaultWithdraw};
use rover::msg::execute::CallbackMsg;

use crate::helpers::{assert_err, uatom_info, uosmo_info, AccountToFund, MockEnv, VaultTestInfo};

pub mod helpers;

#[test]
fn test_only_owner_of_token_can_withdraw_from_vault() {
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().build().unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.update_credit_account(
        &account_id,
        &bad_guy,
        vec![VaultWithdraw {
            vault: VaultBase::new("some_vault".to_string()),
            amount: STARTING_VAULT_SHARES,
        }],
        &[],
    );

    assert_err(
        res,
        NotTokenOwner {
            user: bad_guy.into(),
            account_id,
        },
    )
}

#[test]
fn test_can_only_take_action_on_whitelisted_vaults() {
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().build().unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![VaultWithdraw {
            vault: VaultBase::new("not_allowed_vault".to_string()),
            amount: STARTING_VAULT_SHARES,
        }],
        &[],
    );

    assert_err(res, NotWhitelisted("not_allowed_vault".to_string()))
}

#[test]
fn test_no_unlocked_vault_coins_to_withdraw() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();

    let leverage_vault = VaultTestInfo {
        denom: "uleverage".to_string(),
        lockup: Some(213231),
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

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(coin(200, uatom.denom)),
            Deposit(coin(200, uosmo.denom)),
            VaultDeposit {
                vault: vault.clone(),
                coins: vec![coin(100, "uatom"), coin(100, "uosmo")],
            },
            VaultWithdraw {
                vault,
                amount: STARTING_VAULT_SHARES,
            },
        ],
        &[coin(200, "uatom"), coin(200, "uosmo")],
    );

    assert_err(
        res,
        ContractError::Overflow(OverflowError {
            operation: Sub,
            operand1: "0".to_string(),
            operand2: STARTING_VAULT_SHARES.to_string(),
        }),
    )
}

#[test]
fn test_force_withdraw_can_only_be_called_by_rover() {
    let leverage_vault = VaultTestInfo {
        denom: "uleverage".to_string(),
        lockup: Some(213231),
        underlying_denoms: vec!["uatom".to_string(), "uosmo".to_string()],
    };

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_vaults(&[leverage_vault.clone()])
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.invoke_callback(
        &user.clone(),
        CallbackMsg::VaultForceWithdraw {
            account_id,
            vault: VaultBase::new(Addr::unchecked(vault.address)),
            amount: STARTING_VAULT_SHARES,
        },
    );
    assert_err(res, ContractError::ExternalInvocation)
}

#[test]
fn test_force_withdraw_breaks_lock() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();

    let leverage_vault = VaultTestInfo {
        denom: "uleverage".to_string(),
        lockup: Some(213231),
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
            Deposit(coin(200, uosmo.denom)),
            VaultDeposit {
                vault: vault.clone(),
                coins: vec![coin(100, "uatom"), coin(100, "uosmo")],
            },
        ],
        &[coin(200, "uatom"), coin(200, "uosmo")],
    )
    .unwrap();

    // Assert token's position
    let res = mock.query_positions(&account_id);
    assert_eq!(res.vaults.len(), 1);
    let v = res.vaults.first().unwrap();
    assert_eq!(v.state.locked, STARTING_VAULT_SHARES);

    mock.invoke_callback(
        &mock.rover.clone(),
        CallbackMsg::VaultForceWithdraw {
            account_id: account_id.clone(),
            vault: VaultBase::new(Addr::unchecked(vault.address)),
            amount: STARTING_VAULT_SHARES,
        },
    )
    .unwrap();

    // Assert token's updated position
    let res = mock.query_positions(&account_id);
    assert_eq!(res.vaults.len(), 0);
    let atom = get_coin("uatom", &res.coins);
    assert_eq!(atom.amount, Uint128::from(200u128));
    let osmo = get_coin("uosmo", &res.coins);
    assert_eq!(osmo.amount, Uint128::from(200u128));

    // Assert Rover indeed has those on hand in the bank
    let atom = mock.query_balance(&mock.rover, "uatom");
    assert_eq!(atom.amount, Uint128::from(200u128));
    let osmo = mock.query_balance(&mock.rover, "uosmo");
    assert_eq!(osmo.amount, Uint128::from(200u128));

    // Assert Rover does not have the vault tokens anymore
    let lp_balance = mock.query_balance(&mock.rover, &leverage_vault.denom);
    assert_eq!(Uint128::zero(), lp_balance.amount);
}

#[test]
fn test_withdraw_with_unlocked_vault_coins() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();

    let leverage_vault = VaultTestInfo {
        denom: "uleverage".to_string(),
        lockup: None,
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
            Deposit(coin(200, uosmo.denom)),
            VaultDeposit {
                vault: vault.clone(),
                coins: vec![coin(100, "uatom"), coin(100, "uosmo")],
            },
        ],
        &[coin(200, "uatom"), coin(200, "uosmo")],
    )
    .unwrap();

    // Assert token's position
    let res = mock.query_positions(&account_id);
    assert_eq!(res.vaults.len(), 1);
    let v = res.vaults.first().unwrap();
    assert_eq!(v.state.unlocked, STARTING_VAULT_SHARES);
    let atom = get_coin("uatom", &res.coins);
    assert_eq!(atom.amount, Uint128::from(100u128));
    let osmo = get_coin("uosmo", &res.coins);
    assert_eq!(osmo.amount, Uint128::from(100u128));

    // Assert Rover's totals
    let atom = mock.query_balance(&mock.rover, "uatom");
    assert_eq!(atom.amount, Uint128::from(100u128));
    let osmo = mock.query_balance(&mock.rover, "uosmo");
    assert_eq!(osmo.amount, Uint128::from(100u128));

    // Assert Rover has the vault tokens
    let lp_balance = mock.query_balance(&mock.rover, &leverage_vault.denom);
    assert_eq!(STARTING_VAULT_SHARES, lp_balance.amount);

    mock.update_credit_account(
        &account_id,
        &user,
        vec![VaultWithdraw {
            vault,
            amount: STARTING_VAULT_SHARES,
        }],
        &[],
    )
    .unwrap();

    // Assert token's updated position
    let res = mock.query_positions(&account_id);
    assert_eq!(res.vaults.len(), 0);
    let atom = get_coin("uatom", &res.coins);
    assert_eq!(atom.amount, Uint128::from(200u128));
    let osmo = get_coin("uosmo", &res.coins);
    assert_eq!(osmo.amount, Uint128::from(200u128));

    // Assert Rover indeed has those on hand in the bank
    let atom = mock.query_balance(&mock.rover, "uatom");
    assert_eq!(atom.amount, Uint128::from(200u128));
    let osmo = mock.query_balance(&mock.rover, "uosmo");
    assert_eq!(osmo.amount, Uint128::from(200u128));

    // Assert Rover does not have the vault tokens anymore
    let lp_balance = mock.query_balance(&mock.rover, &leverage_vault.denom);
    assert_eq!(Uint128::zero(), lp_balance.amount);
}

fn get_coin(denom: &str, coins: &[Coin]) -> Coin {
    coins.iter().find(|cv| cv.denom == denom).unwrap().clone()
}
