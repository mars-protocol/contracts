use cosmwasm_std::OverflowOperation::Sub;
use cosmwasm_std::StdError::NotFound;
use cosmwasm_std::{Addr, Decimal, OverflowError, Uint128};

use mock_oracle::msg::CoinPrice;
use rover::adapters::vault::VaultBase;
use rover::error::ContractError;
use rover::msg::execute::Action::{
    Borrow, Deposit, EnterVault, LiquidateVault, RequestVaultUnlock,
};
use rover::traits::IntoDecimal;

use crate::helpers::{
    assert_err, get_coin, get_debt, locked_vault_info, lp_token_info, uatom_info, ujake_info,
    unlocked_vault_info, uosmo_info, AccountToFund, MockEnv,
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
        ContractError::Std(NotFound {
            kind: "rover::adapters::vault::amount::VaultPositionAmount".to_string(),
        }),
    )
}

#[test]
fn test_liquidatee_is_not_liquidatable() {
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![lp_token.to_coin(300)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(lp_token.to_coin(200)),
            EnterVault {
                vault,
                coin: lp_token.to_coin(200),
            },
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![LiquidateVault {
            liquidatee_account_id: liquidatee_account_id.clone(),
            debt_coin: lp_token.to_coin(10),
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
    let lp_token = lp_token_info();
    let ujake = ujake_info();
    let leverage_vault = unlocked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone(), ujake.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![lp_token.to_coin(300)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(lp_token.to_coin(200)),
            EnterVault {
                vault,
                coin: lp_token.to_coin(200),
            },
            Borrow(ujake.to_coin(175)),
        ],
        &[lp_token.to_coin(200)],
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
    let lp_token = lp_token_info();
    let ujake = ujake_info();
    let leverage_vault = unlocked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let liquidator = Addr::unchecked("liquidator");

    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone(), ujake.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![lp_token.to_coin(300)],
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
            Deposit(lp_token.to_coin(200)),
            EnterVault {
                vault,
                coin: lp_token.to_coin(200),
            },
            Borrow(ujake.to_coin(175)),
        ],
        &[lp_token.to_coin(200)],
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
    assert_eq!(vault_balance, Uint128::new(893_660)); // 1M - 106_340

    assert_eq!(position.coins.len(), 1);
    let jake_balance = get_coin("ujake", &position.coins);
    assert_eq!(jake_balance.amount, Uint128::new(175));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("ujake", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(166));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.coins.len(), 1);
    assert_eq!(position.debts.len(), 0);
    let lp = get_coin(&lp_token.denom, &position.coins);
    assert_eq!(lp.amount, Uint128::new(21));
}

#[test]
fn test_liquidate_locked_vault() {
    let lp_token = lp_token_info();
    let atom = uatom_info();
    let leverage_vault = locked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let liquidator = Addr::unchecked("liquidator");

    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone(), atom.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![lp_token.to_coin(300)],
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: vec![atom.to_coin(35)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(lp_token.to_coin(80)),
            EnterVault {
                vault,
                coin: lp_token.to_coin(80),
            },
            Borrow(atom.to_coin(700)),
        ],
        &[lp_token.to_coin(80)],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: atom.denom.clone(),
        price: Uint128::new(20).to_dec().unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(atom.to_coin(35)),
            LiquidateVault {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: atom.to_coin(35),
                request_vault: VaultBase::new(mock.get_vault(&leverage_vault).address),
            },
        ],
        &[atom.to_coin(35)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.vaults.len(), 1);
    let vault_amount = position.vaults.first().unwrap().amount.clone();
    // 1M - 930,474 vault tokens liquidated = 69,526
    assert_eq!(vault_amount.locked(), Uint128::new(69_526));
    assert_eq!(vault_amount.unlocking().positions().len(), 0);
    assert_eq!(vault_amount.unlocked(), Uint128::zero());

    assert_eq!(position.coins.len(), 1);
    let atom_balance = get_coin("uatom", &position.coins);
    assert_eq!(atom_balance.amount, Uint128::new(700));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(666)); // 701 - 35

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.coins.len(), 1);
    assert_eq!(position.debts.len(), 0);
    let lp_balance = get_coin(&lp_token.denom, &position.coins);
    assert_eq!(lp_balance.amount, Uint128::new(74));
}

#[test]
fn test_liquidate_unlocking_priority() {
    let lp_token = lp_token_info();
    let ujake = ujake_info();
    let leverage_vault = locked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let liquidator = Addr::unchecked("liquidator");

    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone(), ujake.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![lp_token.to_coin(300)],
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: vec![ujake.to_coin(100)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(lp_token.to_coin(200)),
            EnterVault {
                vault: vault.clone(),
                coin: lp_token.to_coin(200),
            },
            Borrow(ujake.to_coin(175)),
            RequestVaultUnlock {
                vault,
                amount: Uint128::new(100_000),
            },
        ],
        &[lp_token.to_coin(200)],
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

    // Assert only unlocking position liquidated
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.vaults.len(), 1);
    let vault_amount = position.vaults.first().unwrap().amount.clone();
    assert_eq!(vault_amount.unlocked(), Uint128::zero());
    assert_eq!(vault_amount.unlocking().positions().len(), 0);
    assert_eq!(vault_amount.locked(), Uint128::new(900_000));

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

    // Assert locked positions can now be liquidated
    let position = mock.query_positions(&liquidatee_account_id);
    let vault_amount = position.vaults.first().unwrap().amount.clone();
    assert!(vault_amount.locked() < Uint128::new(900_000));
}

#[test]
fn test_liquidate_unlocking_liquidation_order() {
    let lp_token = lp_token_info();
    let ujake = ujake_info();
    let leverage_vault = locked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let liquidator = Addr::unchecked("liquidator");

    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone(), ujake.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![lp_token.to_coin(300)],
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
            Deposit(lp_token.to_coin(200)),
            EnterVault {
                vault: vault.clone(),
                coin: lp_token.to_coin(200),
            },
            Borrow(ujake.to_coin(175)),
            RequestVaultUnlock {
                vault: vault.clone(),
                amount: Uint128::new(10_000),
            },
            RequestVaultUnlock {
                vault: vault.clone(),
                amount: Uint128::new(50_000),
            },
            RequestVaultUnlock {
                vault: vault.clone(),
                amount: Uint128::new(100_000),
            },
            RequestVaultUnlock {
                vault,
                amount: Uint128::new(840_000),
            },
        ],
        &[lp_token.to_coin(200)],
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
    assert_eq!(vault_amount.locked(), Uint128::zero());

    // Total liquidated:                   21 LP tokens
    // First bucket drained:               -2 (all)
    // Second bucket drained:              -10 (all)
    // Third bucket partially liquidated:  -10 (out of 20)
    // Fourth bucket retained:             -0 (out of 168)
    assert_eq!(vault_amount.unlocking().positions().len(), 2);
    assert_eq!(
        vault_amount
            .unlocking()
            .positions()
            .first()
            .unwrap()
            .coin
            .amount,
        Uint128::new(10)
    );
    assert_eq!(
        vault_amount
            .unlocking()
            .positions()
            .get(1)
            .unwrap()
            .coin
            .amount,
        Uint128::new(168)
    );

    assert_eq!(position.coins.len(), 1);
    let jake_balance = get_coin("ujake", &position.coins);
    assert_eq!(jake_balance.amount, Uint128::new(175));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("ujake", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(166));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.coins.len(), 1);
    assert_eq!(position.debts.len(), 0);
    let osmo_balance = get_coin(&lp_token.denom, &position.coins);
    assert_eq!(osmo_balance.amount, Uint128::new(22));
}

// NOTE: liquidation calculation+adjustments are quite complex, full cases in test_liquidate_coin.rs
#[test]
fn test_liquidation_calculation_adjustment() {
    let lp_token = lp_token_info();
    let ujake = ujake_info();
    let leverage_vault = unlocked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let liquidator = Addr::unchecked("liquidator");

    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone(), ujake.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![lp_token.to_coin(300)],
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: vec![ujake.to_coin(500)],
        })
        .max_close_factor(Decimal::from_atomics(87u128, 2).unwrap())
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(lp_token.to_coin(200)),
            EnterVault {
                vault,
                coin: lp_token.to_coin(200),
            },
            Borrow(ujake.to_coin(175)),
        ],
        &[lp_token.to_coin(200)],
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
            Deposit(ujake.to_coin(500)),
            LiquidateVault {
                liquidatee_account_id: liquidatee_account_id.clone(),
                // Given the request vault balance, this debt payment is too high.
                // It will be adjusted to 94, the max given the request vault value
                debt_coin: ujake.to_coin(500),
                request_vault: VaultBase::new(mock.get_vault(&leverage_vault).address),
            },
        ],
        &[ujake.to_coin(500)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.vaults.len(), 1);
    let vault_balance = position.vaults.first().unwrap().amount.unlocked();
    assert_eq!(vault_balance, Uint128::new(405)); // Vault position liquidated by 99%

    assert_eq!(position.coins.len(), 1);
    let jake_balance = get_coin("ujake", &position.coins);
    assert_eq!(jake_balance.amount, Uint128::new(175));

    assert_eq!(position.debts.len(), 1);
    let ujake_debt = get_debt("ujake", &position.debts);
    assert_eq!(ujake_debt.amount, Uint128::new(82));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.coins.len(), 2);
    let osmo_balance = get_coin("ujake", &position.coins);
    assert_eq!(osmo_balance.amount, Uint128::new(406));
    let atom_balance = get_coin(&lp_token.denom, &position.coins);
    assert_eq!(atom_balance.amount, Uint128::new(199));
    assert_eq!(position.debts.len(), 0);
}
