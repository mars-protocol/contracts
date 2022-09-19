use cosmwasm_std::OverflowOperation::Sub;
use cosmwasm_std::{coin, coins, Addr, OverflowError, Uint128};

use mock_vault::contract::STARTING_VAULT_SHARES;
use rover::adapters::VaultBase;
use rover::error::ContractError;
use rover::msg::execute::Action::{Deposit, VaultDeposit};

use crate::helpers::{assert_err, uatom_info, uosmo_info, AccountToFund, MockEnv, VaultTestInfo};

pub mod helpers;

#[test]
fn test_only_account_owner_can_take_action() {
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().build().unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.update_credit_account(
        &account_id,
        &bad_guy,
        vec![VaultDeposit {
            vault: VaultBase::new("xyz".to_string()),
            coins: vec![],
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
fn test_all_deposit_coins_are_whitelisted() {
    let uatom = uatom_info();
    let leverage_vault = VaultTestInfo {
        lp_token_denom: "uleverage".to_string(),
        lockup: None,
        asset_denoms: vec!["uatom".to_string(), "uosmo".to_string()],
    };

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uatom])
        .allowed_vaults(&[leverage_vault.clone()])
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![VaultDeposit {
            vault,
            coins: vec![coin(200, "uatom"), coin(400, "uosmo")],
        }],
        &[],
    );

    assert_err(res, ContractError::NotWhitelisted("uosmo".to_string()));
}

#[test]
fn test_vault_is_whitelisted() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();

    let leverage_vault = VaultTestInfo {
        lp_token_denom: "uleverage".to_string(),
        lockup: None,
        asset_denoms: vec!["uatom".to_string(), "uosmo".to_string()],
    };

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uatom, uosmo])
        .allowed_vaults(&[leverage_vault])
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![VaultDeposit {
            vault: VaultBase::new("unknown_vault".to_string()),
            coins: coins(200, "uatom"),
        }],
        &[],
    );

    assert_err(
        res,
        ContractError::NotWhitelisted("unknown_vault".to_string()),
    );
}

#[test]
fn test_deposited_coins_match_vault_requirements() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();

    let leverage_vault = VaultTestInfo {
        lp_token_denom: "uleverage".to_string(),
        lockup: None,
        asset_denoms: vec!["uatom".to_string(), "ujake".to_string()],
    };

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uatom, uosmo])
        .allowed_vaults(&[leverage_vault.clone()])
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![VaultDeposit {
            vault: mock.get_vault(&leverage_vault),
            coins: vec![coin(200, "uatom"), coin(200, "uosmo")],
        }],
        &[],
    );

    assert_err(
        res,
        ContractError::RequirementsNotMet(
            "Required assets: uatom, ujake -- do not match given assets: uatom, uosmo".to_string(),
        ),
    );
}

#[test]
fn test_fails_if_not_enough_funds_for_deposit() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();

    let leverage_vault = VaultTestInfo {
        lp_token_denom: "uleverage".to_string(),
        lockup: None,
        asset_denoms: vec!["uatom".to_string(), "uosmo".to_string()],
    };

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uatom, uosmo])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(300, "uatom"), coin(500, "uosmo")],
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![VaultDeposit {
            vault: mock.get_vault(&leverage_vault),
            coins: vec![coin(200, "uatom"), coin(200, "uosmo")],
        }],
        &[],
    );

    assert_err(
        res,
        ContractError::Overflow(OverflowError {
            operation: Sub,
            operand1: "0".to_string(),
            operand2: "200".to_string(),
        }),
    );
}

#[test]
fn test_successful_deposit_into_locked_vault() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();

    let leverage_vault = VaultTestInfo {
        lp_token_denom: "uleverage".to_string(),
        lockup: Some(1_209_600u64),
        asset_denoms: vec!["uatom".to_string(), "uosmo".to_string()],
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
    let balance = mock.query_total_vault_coin_balance(&vault);
    assert_eq!(balance, Uint128::zero());

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
        ],
        &[coin(200, "uatom"), coin(400, "uosmo")],
    )
    .unwrap();

    let lp_balance = mock.query_balance(&mock.rover, &leverage_vault.lp_token_denom);
    assert_eq!(STARTING_VAULT_SHARES, lp_balance.amount);

    let res = mock.query_position(&account_id);
    assert_eq!(res.vault_positions.len(), 1);
    assert_eq!(
        STARTING_VAULT_SHARES,
        res.vault_positions.first().unwrap().position.locked
    );
    assert_eq!(
        Uint128::zero(),
        res.vault_positions.first().unwrap().position.unlocked
    );

    let assets =
        mock.query_preview_redeem(&vault, res.vault_positions.first().unwrap().position.locked);

    let osmo_withdraw = assets.iter().find(|coin| coin.denom == "uosmo").unwrap();
    assert_eq!(osmo_withdraw.amount, Uint128::new(120));
    let atom_withdraw = assets.iter().find(|coin| coin.denom == "uatom").unwrap();
    assert_eq!(atom_withdraw.amount, Uint128::new(23));

    let balance = mock.query_total_vault_coin_balance(&vault);
    assert_eq!(balance, STARTING_VAULT_SHARES);

    let vault_token_balance = mock.query_balance(&mock.rover, &leverage_vault.lp_token_denom);
    assert_eq!(vault_token_balance.amount, STARTING_VAULT_SHARES)
}

#[test]
fn test_successful_deposit_into_unlocked_vault() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();

    let leverage_vault = VaultTestInfo {
        lp_token_denom: "uleverage".to_string(),
        lockup: None,
        asset_denoms: vec!["uatom".to_string(), "uosmo".to_string()],
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
        ],
        &[coin(200, "uatom"), coin(400, "uosmo")],
    )
    .unwrap();

    let lp_balance = mock.query_balance(&mock.rover, &leverage_vault.lp_token_denom);
    assert_eq!(STARTING_VAULT_SHARES, lp_balance.amount);

    let res = mock.query_position(&account_id);
    assert_eq!(res.vault_positions.len(), 1);
    assert_eq!(
        STARTING_VAULT_SHARES,
        res.vault_positions.first().unwrap().position.unlocked
    );
    assert_eq!(
        Uint128::zero(),
        res.vault_positions.first().unwrap().position.locked
    );

    let assets = mock.query_preview_redeem(
        &vault,
        res.vault_positions.first().unwrap().position.unlocked,
    );

    let osmo_withdraw = assets.iter().find(|coin| coin.denom == "uosmo").unwrap();
    assert_eq!(osmo_withdraw.amount, Uint128::new(120));
    let atom_withdraw = assets.iter().find(|coin| coin.denom == "uatom").unwrap();
    assert_eq!(atom_withdraw.amount, Uint128::new(23));

    let balance = mock.query_total_vault_coin_balance(&vault);
    assert_eq!(balance, STARTING_VAULT_SHARES);

    let vault_token_balance = mock.query_balance(&mock.rover, &leverage_vault.lp_token_denom);
    assert_eq!(vault_token_balance.amount, STARTING_VAULT_SHARES)
}
