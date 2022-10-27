use cosmwasm_std::OverflowOperation::Sub;
use cosmwasm_std::StdError::NotFound;
use cosmwasm_std::{Addr, OverflowError, Uint128};

use mock_oracle::msg::CoinPrice;
use rover::adapters::vault::VaultBase;
use rover::error::ContractError;
use rover::msg::execute::Action::{
    Borrow, Deposit, LiquidateVault, VaultDeposit, VaultRequestUnlock,
};
use rover::traits::IntoDecimal;

use crate::helpers::{
    assert_err, get_coin, get_debt, locked_vault_info, uatom_info, ujake_info, unlocked_vault_info,
    uosmo_info, AccountToFund, MockEnv,
};

pub mod helpers;

// NOTE: Vault liquidation scenarios spreadsheet:
// https://docs.google.com/spreadsheets/d/1rXa_8eKbtp1wQ0Mm1Rny7QzSLsko9D7UQTtO7NrAssA/edit#gid=2127757089

#[test]
fn test_liquidatee_must_have_the_request_vault_position() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();
    let leverage_vault = unlocked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uatom.clone(), uosmo.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![uatom.to_coin(300), uosmo.to_coin(500)],
        })
        .build()
        .unwrap();

    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![Deposit(uatom.to_coin(200)), Deposit(uosmo.to_coin(400))],
        &[uatom.to_coin(200), uosmo.to_coin(400)],
    )
    .unwrap();

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![LiquidateVault {
            liquidatee_account_id: liquidatee_account_id.clone(),
            debt_coin: uatom.to_coin(10),
            request_vault: VaultBase::new(mock.get_vault(&leverage_vault).address),
        }],
        &[],
    );

    assert_err(
        res,
        ContractError::Std(NotFound { kind: "rover::adapters::vault::amount::VaultPositionAmountBase<rover::adapters::vault::amount::VaultAmount, rover::adapters::vault::amount::LockingVaultAmount>".to_string() }),
    )
}

#[test]
fn test_liquidatee_is_not_liquidatable() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();
    let leverage_vault = unlocked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uatom.clone(), uosmo.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![uatom.to_coin(300), uosmo.to_coin(500)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uatom.to_coin(200)),
            Deposit(uosmo.to_coin(400)),
            VaultDeposit {
                vault,
                coins: vec![uatom.to_coin(200), uosmo.to_coin(400)],
            },
        ],
        &[uatom.to_coin(200), uosmo.to_coin(400)],
    )
    .unwrap();

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![LiquidateVault {
            liquidatee_account_id: liquidatee_account_id.clone(),
            debt_coin: uatom.to_coin(10),
            request_vault: VaultBase::new(mock.get_vault(&leverage_vault).address),
        }],
        &[],
    );

    assert_err(
        res,
        ContractError::NotLiquidatable {
            account_id: liquidatee_account_id,
            lqdt_health_factor: "n/a".to_string(),
        },
    )
}

#[test]
fn test_liquidator_does_not_have_debt_coin_in_credit_account() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();
    let ujake = ujake_info();
    let leverage_vault = unlocked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uatom.clone(), uosmo.clone(), ujake.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![uatom.to_coin(300), uosmo.to_coin(500)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uatom.to_coin(300)),
            Deposit(uosmo.to_coin(400)),
            VaultDeposit {
                vault,
                coins: vec![uatom.to_coin(200), uosmo.to_coin(400)],
            },
            Borrow(ujake.to_coin(175)),
        ],
        &[uatom.to_coin(300), uosmo.to_coin(400)],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: ujake.denom.clone(),
        price: Uint128::new(20).to_dec().unwrap(),
    });

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![LiquidateVault {
            liquidatee_account_id: liquidatee_account_id.clone(),
            debt_coin: ujake.to_coin(10),
            request_vault: VaultBase::new(mock.get_vault(&leverage_vault).address),
        }],
        &[],
    );

    assert_err(
        res,
        ContractError::Overflow(OverflowError {
            operation: Sub,
            operand1: "0".to_string(),
            operand2: "10".to_string(),
        }),
    )
}

#[test]
fn test_liquidate_unlocked_vault() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();
    let ujake = ujake_info();
    let leverage_vault = unlocked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let liquidator = Addr::unchecked("liquidator");

    let mut mock = MockEnv::new()
        .allowed_coins(&[uatom.clone(), uosmo.clone(), ujake.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![uatom.to_coin(300), uosmo.to_coin(500)],
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: vec![ujake.to_coin(10)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uatom.to_coin(300)),
            Deposit(uosmo.to_coin(400)),
            VaultDeposit {
                vault,
                coins: vec![uatom.to_coin(200), uosmo.to_coin(400)],
            },
            Borrow(ujake.to_coin(175)),
        ],
        &[uatom.to_coin(300), uosmo.to_coin(400)],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: ujake.denom.clone(),
        price: Uint128::new(20).to_dec().unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(ujake.to_coin(10)),
            LiquidateVault {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: ujake.to_coin(10),
                request_vault: VaultBase::new(mock.get_vault(&leverage_vault).address),
            },
        ],
        &[ujake.to_coin(10)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.vaults.len(), 1);
    let vault_balance = position.vaults.first().unwrap().amount.unlocked();
    assert_eq!(vault_balance, Uint128::new(300_000)); // Vault position liquidated by 70%

    assert_eq!(position.coins.len(), 2);
    let jake_balance = get_coin("ujake", &position.coins);
    assert_eq!(jake_balance.amount, Uint128::new(175));
    let atom_balance = get_coin("uatom", &position.coins);
    assert_eq!(atom_balance.amount, Uint128::new(100));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("ujake", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(166));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.coins.len(), 2);
    assert_eq!(position.debts.len(), 0);
    let osmo_balance = get_coin("uosmo", &position.coins);
    assert_eq!(osmo_balance.amount, Uint128::new(280));
    let atom_balance = get_coin("uatom", &position.coins);
    assert_eq!(atom_balance.amount, Uint128::new(140));
}

#[test]
fn test_liquidate_locked_vault() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();
    let ujake = ujake_info();
    let leverage_vault = locked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let liquidator = Addr::unchecked("liquidator");

    let mut mock = MockEnv::new()
        .allowed_coins(&[uatom.clone(), uosmo.clone(), ujake.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![uatom.to_coin(300), uosmo.to_coin(500)],
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: vec![ujake.to_coin(10)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uatom.to_coin(300)),
            Deposit(uosmo.to_coin(400)),
            VaultDeposit {
                vault: vault.clone(),
                coins: vec![uatom.to_coin(200), uosmo.to_coin(400)],
            },
            Borrow(ujake.to_coin(175)),
            VaultRequestUnlock {
                vault,
                amount: Uint128::new(100_000),
            },
        ],
        &[uatom.to_coin(300), uosmo.to_coin(400)],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: ujake.denom.clone(),
        price: Uint128::new(20).to_dec().unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(ujake.to_coin(10)),
            LiquidateVault {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: ujake.to_coin(10),
                request_vault: VaultBase::new(mock.get_vault(&leverage_vault).address),
            },
        ],
        &[ujake.to_coin(10)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.vaults.len(), 1);
    let vault_amount = position.vaults.first().unwrap().amount.clone();
    // Vault position liquidated by 70%. Unlocking first, then locked bucket.
    assert_eq!(vault_amount.unlocking().len(), 0);
    assert_eq!(vault_amount.locked(), Uint128::new(300_000));

    assert_eq!(position.coins.len(), 2);
    let jake_balance = get_coin("ujake", &position.coins);
    assert_eq!(jake_balance.amount, Uint128::new(175));
    let atom_balance = get_coin("uatom", &position.coins);
    assert_eq!(atom_balance.amount, Uint128::new(100));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("ujake", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(166));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.coins.len(), 2);
    assert_eq!(position.debts.len(), 0);
    let osmo_balance = get_coin("uosmo", &position.coins);
    assert_eq!(osmo_balance.amount, Uint128::new(280));
    let atom_balance = get_coin("uatom", &position.coins);
    assert_eq!(atom_balance.amount, Uint128::new(140));
}

#[test]
fn test_liquidate_unlocking_priority() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();
    let ujake = ujake_info();
    let leverage_vault = locked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let liquidator = Addr::unchecked("liquidator");

    let mut mock = MockEnv::new()
        .allowed_coins(&[uatom.clone(), uosmo.clone(), ujake.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![uatom.to_coin(300), uosmo.to_coin(500)],
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: vec![ujake.to_coin(10)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uatom.to_coin(300)),
            Deposit(uosmo.to_coin(400)),
            VaultDeposit {
                vault: vault.clone(),
                coins: vec![uatom.to_coin(200), uosmo.to_coin(400)],
            },
            Borrow(ujake.to_coin(175)),
            VaultRequestUnlock {
                vault: vault.clone(),
                amount: Uint128::new(10_000),
            },
            VaultRequestUnlock {
                vault: vault.clone(),
                amount: Uint128::new(200_000),
            },
            VaultRequestUnlock {
                vault,
                amount: Uint128::new(700_000),
            },
        ],
        &[uatom.to_coin(300), uosmo.to_coin(400)],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: ujake.denom.clone(),
        price: Uint128::new(20).to_dec().unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(ujake.to_coin(10)),
            LiquidateVault {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: ujake.to_coin(10),
                request_vault: VaultBase::new(mock.get_vault(&leverage_vault).address),
            },
        ],
        &[ujake.to_coin(10)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.vaults.len(), 1);
    let vault_amount = position.vaults.first().unwrap().amount.clone();
    assert_eq!(vault_amount.unlocked(), Uint128::zero());
    assert_eq!(vault_amount.locked(), Uint128::new(90_000));
    assert_eq!(vault_amount.unlocking().len(), 1);
    assert_eq!(
        vault_amount.unlocking().first().unwrap().amount,
        Uint128::new(210_000)
    );

    assert_eq!(position.coins.len(), 2);
    let jake_balance = get_coin("ujake", &position.coins);
    assert_eq!(jake_balance.amount, Uint128::new(175));
    let atom_balance = get_coin("uatom", &position.coins);
    assert_eq!(atom_balance.amount, Uint128::new(100));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("ujake", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(166));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.coins.len(), 2);
    assert_eq!(position.debts.len(), 0);
    let osmo_balance = get_coin("uosmo", &position.coins);
    assert_eq!(osmo_balance.amount, Uint128::new(280));
    let atom_balance = get_coin("uatom", &position.coins);
    assert_eq!(atom_balance.amount, Uint128::new(140));
}

// NOTE: liquidation calculation+adjustments are quite complex, full cases in test_liquidate_coin.rs
#[test]
fn test_liquidation_calculation_adjustment() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();
    let ujake = ujake_info();
    let leverage_vault = unlocked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let liquidator = Addr::unchecked("liquidator");

    let mut mock = MockEnv::new()
        .allowed_coins(&[uatom.clone(), uosmo.clone(), ujake.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![uatom.to_coin(300), uosmo.to_coin(500)],
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: vec![ujake.to_coin(50)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uatom.to_coin(300)),
            Deposit(uosmo.to_coin(400)),
            VaultDeposit {
                vault,
                coins: vec![uatom.to_coin(200), uosmo.to_coin(400)],
            },
            Borrow(ujake.to_coin(175)),
        ],
        &[uatom.to_coin(300), uosmo.to_coin(400)],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: ujake.denom.clone(),
        price: Uint128::new(20).to_dec().unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(ujake.to_coin(50)),
            LiquidateVault {
                liquidatee_account_id: liquidatee_account_id.clone(),
                // Given the request vault balance, this debt payment is too high.
                // It will be adjusted to 14, the max given the request vault value
                debt_coin: ujake.to_coin(50),
                request_vault: VaultBase::new(mock.get_vault(&leverage_vault).address),
            },
        ],
        &[ujake.to_coin(50)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.vaults.len(), 1);
    let vault_balance = position.vaults.first().unwrap().amount.unlocked();
    assert_eq!(vault_balance, Uint128::new(20_000)); // Vault position liquidated by 98%

    assert_eq!(position.coins.len(), 2);
    let jake_balance = get_coin("ujake", &position.coins);
    assert_eq!(jake_balance.amount, Uint128::new(175));
    let atom_balance = get_coin("uatom", &position.coins);
    assert_eq!(atom_balance.amount, Uint128::new(100));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("ujake", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(162));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.coins.len(), 3);
    let osmo_balance = get_coin("ujake", &position.coins);
    assert_eq!(osmo_balance.amount, Uint128::new(36));
    let osmo_balance = get_coin("uosmo", &position.coins);
    assert_eq!(osmo_balance.amount, Uint128::new(392));
    let atom_balance = get_coin("uatom", &position.coins);
    assert_eq!(atom_balance.amount, Uint128::new(196));
    assert_eq!(position.debts.len(), 0);
}
