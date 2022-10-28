use cosmwasm_std::OverflowOperation::Sub;
use cosmwasm_std::{coin, Addr, OverflowError, Uint128};

use mock_vault::contract::STARTING_VAULT_SHARES;
use rover::adapters::vault::VaultBase;
use rover::error::ContractError;
use rover::msg::execute::Action::{Deposit, EnterVault};

use crate::helpers::{
    assert_err, locked_vault_info, lp_token_info, uatom_info, unlocked_vault_info, uosmo_info,
    AccountToFund, MockEnv,
};

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
        vec![EnterVault {
            vault: VaultBase::new("xyz".to_string()),
            coin: coin(1, "uosmo"),
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
fn test_deposit_denom_is_whitelisted() {
    let lp_token = lp_token_info();
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
        vec![EnterVault {
            vault,
            coin: coin(200, lp_token.denom.clone()),
        }],
        &[],
    );

    assert_err(res, ContractError::NotWhitelisted(lp_token.denom));
}

#[test]
fn test_vault_is_whitelisted() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();
    let leverage_vault = unlocked_vault_info();

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
        vec![EnterVault {
            vault: VaultBase::new("unknown_vault".to_string()),
            coin: coin(200, "uatom"),
        }],
        &[],
    );

    assert_err(
        res,
        ContractError::NotWhitelisted("unknown_vault".to_string()),
    );
}

#[test]
fn test_deposited_coin_matches_vault_requirements() {
    let uatom = uatom_info();
    let leverage_vault = unlocked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uatom])
        .allowed_vaults(&[leverage_vault.clone()])
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![EnterVault {
            vault: mock.get_vault(&leverage_vault),
            coin: coin(200, "uatom"),
        }],
        &[],
    );

    assert_err(
        res,
        ContractError::RequirementsNotMet(
            "Required coin: ugamm22 -- does not match given coin: uatom".to_string(),
        ),
    );
}

#[test]
fn test_fails_if_not_enough_funds_for_deposit() {
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

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

    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![EnterVault {
            vault: mock.get_vault(&leverage_vault),
            coin: coin(200, lp_token.denom),
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
    let balance = mock.query_total_vault_coin_balance(&vault);
    assert_eq!(balance, Uint128::zero());

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(200)),
            EnterVault {
                vault: vault.clone(),
                coin: lp_token.to_coin(23),
            },
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    let lp_balance = mock.query_balance(&mock.rover, &leverage_vault.vault_token_denom);
    assert_eq!(STARTING_VAULT_SHARES, lp_balance.amount);

    let res = mock.query_positions(&account_id);
    assert_eq!(res.vaults.len(), 1);
    assert_eq!(
        STARTING_VAULT_SHARES,
        res.vaults.first().unwrap().amount.locked()
    );
    assert_eq!(
        Uint128::zero(),
        res.vaults.first().unwrap().amount.unlocked()
    );

    let assets = mock.query_preview_redeem(&vault, res.vaults.first().unwrap().amount.locked());
    assert_eq!(assets.coin.denom, lp_token.denom);
    assert_eq!(assets.coin.amount, Uint128::new(23));

    let balance = mock.query_total_vault_coin_balance(&vault);
    assert_eq!(balance, STARTING_VAULT_SHARES);

    let vault_token_balance = mock.query_balance(&mock.rover, &leverage_vault.vault_token_denom);
    assert_eq!(vault_token_balance.amount, STARTING_VAULT_SHARES)
}

#[test]
fn test_successful_deposit_into_unlocked_vault() {
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

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
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    let lp_balance = mock.query_balance(&mock.rover, &leverage_vault.vault_token_denom);
    assert_eq!(STARTING_VAULT_SHARES, lp_balance.amount);

    let res = mock.query_positions(&account_id);
    assert_eq!(res.vaults.len(), 1);
    assert_eq!(
        STARTING_VAULT_SHARES,
        res.vaults.first().unwrap().amount.unlocked()
    );
    assert_eq!(Uint128::zero(), res.vaults.first().unwrap().amount.locked());

    let assets = mock.query_preview_redeem(&vault, res.vaults.first().unwrap().amount.unlocked());
    assert_eq!(assets.coin.denom, lp_token.denom);
    assert_eq!(assets.coin.amount, Uint128::new(23));

    let balance = mock.query_total_vault_coin_balance(&vault);
    assert_eq!(balance, STARTING_VAULT_SHARES);

    let vault_token_balance = mock.query_balance(&mock.rover, &leverage_vault.vault_token_denom);
    assert_eq!(vault_token_balance.amount, STARTING_VAULT_SHARES)
}

#[test]
fn test_vault_deposit_must_be_under_cap() {
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![lp_token.to_coin(3_300_000)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let account_id = mock.create_credit_account(&user).unwrap();

    // Vault deposit A ✅
    //   new total value = 6_911_800
    //   left to deposit = 5_433_200
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(700_000)),
            EnterVault {
                vault: vault.clone(),
                coin: lp_token.to_coin(700_000),
            },
        ],
        &[lp_token.to_coin(700_000)],
    )
    .unwrap();

    // Vault deposit B ✅
    //   new total value = 7_899_200
    //   left to deposit = 4_445_800
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(100_000)),
            EnterVault {
                vault: vault.clone(),
                coin: lp_token.to_coin(100_000),
            },
        ],
        &[lp_token.to_coin(100_000)],
    )
    .unwrap();

    // Vault deposit C 🚫
    //   new total value = 32_584_200
    //   left to deposit = -20_239_200
    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(2_500_000)),
            EnterVault {
                vault,
                coin: lp_token.to_coin(2_500_000),
            },
        ],
        &[lp_token.to_coin(2_500_000)],
    );

    assert_err(
        res,
        ContractError::AboveVaultDepositCap {
            new_value: "32584199.999999999998984572".to_string(),
            maximum: "12345000".to_string(),
        },
    );
}
