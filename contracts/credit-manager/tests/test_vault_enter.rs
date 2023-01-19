use cosmwasm_std::{
    coin, Addr, Decimal, OverflowError, OverflowOperation::Sub, StdError::NotFound, Uint128,
};
use mars_mock_vault::contract::STARTING_VAULT_SHARES;
use mars_rover::{
    adapters::vault::VaultBase,
    error::ContractError,
    msg::execute::{
        Action::{Deposit, EnterVault},
        ActionAmount, ActionCoin,
    },
};

use crate::helpers::{
    assert_err, locked_vault_info, lp_token_info, uatom_info, unlocked_vault_info, uosmo_info,
    AccountToFund, MockEnv, VaultTestInfo,
};

pub mod helpers;

#[test]
fn only_account_owner_can_take_action() {
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().build().unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.update_credit_account(
        &account_id,
        &bad_guy,
        vec![EnterVault {
            vault: VaultBase::new("xyz".to_string()),
            coin: ActionCoin {
                denom: "uosmo".to_string(),
                amount: ActionAmount::Exact(Uint128::new(1)),
            },
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
fn deposit_denom_is_whitelisted() {
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().vault_configs(&[leverage_vault.clone()]).build().unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![EnterVault {
            vault,
            coin: lp_token.to_action_coin(200),
        }],
        &[],
    );

    assert_err(res, ContractError::NotWhitelisted(lp_token.denom));
}

#[test]
fn vault_is_whitelisted() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();
    let leverage_vault = VaultTestInfo {
        vault_token_denom: "uleverage".to_string(),
        lockup: None,
        base_token_denom: uatom.denom.clone(),
        deposit_cap: coin(10_000_000, "uusdc"),
        max_ltv: Decimal::from_atomics(6u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(7u128, 1).unwrap(),
        whitelisted: false,
    };

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uatom.clone(), uosmo])
        .vault_configs(&[leverage_vault.clone()])
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let vault = mock.get_vault(&leverage_vault);

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![EnterVault {
            vault: vault.clone(),
            coin: uatom.to_action_coin(200),
        }],
        &[],
    );

    assert_err(res, ContractError::NotWhitelisted(vault.address));
}

#[test]
fn deposited_coin_matches_vault_requirements() {
    let uatom = uatom_info();
    let leverage_vault = unlocked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uatom.clone()])
        .vault_configs(&[leverage_vault.clone()])
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![EnterVault {
            vault: mock.get_vault(&leverage_vault),
            coin: uatom.to_action_coin(200),
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
fn fails_if_not_enough_funds_for_implied_deposit() {
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone()])
        .vault_configs(&[leverage_vault.clone()])
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
            coin: lp_token.to_action_coin_full_balance(),
        }],
        &[],
    );

    assert_err(
        res,
        ContractError::Std(NotFound {
            kind: "cosmwasm_std::math::uint128::Uint128".to_string(),
        }),
    );
}

#[test]
fn fails_if_not_enough_funds_for_enumerated_deposit() {
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone()])
        .vault_configs(&[leverage_vault.clone()])
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
            coin: lp_token.to_action_coin(200),
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
fn successful_deposit_into_locked_vault() {
    let lp_token = lp_token_info();
    let leverage_vault = locked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone()])
        .vault_configs(&[leverage_vault.clone()])
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
                coin: lp_token.to_action_coin(23),
            },
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    let lp_balance = mock.query_balance(&mock.rover, &leverage_vault.vault_token_denom);
    assert_eq!(STARTING_VAULT_SHARES, lp_balance.amount);

    let res = mock.query_positions(&account_id);
    assert_eq!(res.vaults.len(), 1);
    assert_eq!(STARTING_VAULT_SHARES, res.vaults.first().unwrap().amount.locked());
    assert_eq!(Uint128::zero(), res.vaults.first().unwrap().amount.unlocked());

    let amount = mock.query_preview_redeem(&vault, res.vaults.first().unwrap().amount.locked());
    assert_eq!(amount, Uint128::new(23));

    let balance = mock.query_total_vault_coin_balance(&vault);
    assert_eq!(balance, STARTING_VAULT_SHARES);

    let vault_token_balance = mock.query_balance(&mock.rover, &leverage_vault.vault_token_denom);
    assert_eq!(vault_token_balance.amount, STARTING_VAULT_SHARES)
}

#[test]
fn successful_deposit_into_unlocked_vault() {
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone()])
        .vault_configs(&[leverage_vault.clone()])
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
                coin: lp_token.to_action_coin(23),
            },
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    let lp_balance = mock.query_balance(&mock.rover, &leverage_vault.vault_token_denom);
    assert_eq!(STARTING_VAULT_SHARES, lp_balance.amount);

    let res = mock.query_positions(&account_id);
    assert_eq!(res.vaults.len(), 1);
    assert_eq!(STARTING_VAULT_SHARES, res.vaults.first().unwrap().amount.unlocked());
    assert_eq!(Uint128::zero(), res.vaults.first().unwrap().amount.locked());

    let amount = mock.query_preview_redeem(&vault, res.vaults.first().unwrap().amount.unlocked());
    assert_eq!(amount, Uint128::new(23));

    let balance = mock.query_total_vault_coin_balance(&vault);
    assert_eq!(balance, STARTING_VAULT_SHARES);

    let vault_token_balance = mock.query_balance(&mock.rover, &leverage_vault.vault_token_denom);
    assert_eq!(vault_token_balance.amount, STARTING_VAULT_SHARES)
}

#[test]
fn vault_deposit_must_be_under_cap() {
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone()])
        .vault_configs(&[leverage_vault.clone()])
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
                coin: lp_token.to_action_coin(700_000),
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
                coin: lp_token.to_action_coin(100_000),
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
                coin: lp_token.to_action_coin(2_500_000),
            },
        ],
        &[lp_token.to_coin(2_500_000)],
    );

    assert_err(
        res,
        ContractError::AboveVaultDepositCap {
            new_value: "32584200".to_string(),
            maximum: "12345000".to_string(),
        },
    );
}

#[test]
fn successful_deposit_with_implied_full_balance_amount() {
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone()])
        .vault_configs(&[leverage_vault.clone()])
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
                coin: lp_token.to_action_coin_full_balance(),
            },
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    // Assert credit account has full balance of LP token
    let res = mock.query_positions(&account_id);
    let amount = mock.query_preview_redeem(&vault, res.vaults.first().unwrap().amount.unlocked());
    assert_eq!(amount, Uint128::new(200));
    assert_eq!(res.deposits.len(), 0);

    // Assert vault indeed has those tokens
    let base_denom =
        mock.query_balance(&Addr::unchecked(vault.address), &leverage_vault.base_token_denom);
    assert_eq!(base_denom.amount, Uint128::new(200))
}
