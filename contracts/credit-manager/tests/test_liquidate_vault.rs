use cosmwasm_std::{
    Addr, Decimal, OverflowError, OverflowOperation::Sub, StdError::NotFound, Uint128,
};
use mars_mock_oracle::msg::CoinPrice;
use mars_rover::{
    adapters::vault::{VaultBase, VaultPositionType},
    error::ContractError,
    msg::execute::{
        Action::{Borrow, Deposit, EnterVault, Liquidate, RequestVaultUnlock},
        LiquidateRequest,
    },
};

use crate::helpers::{
    assert_err, get_coin, get_debt, locked_vault_info, lp_token_info, uatom_info, ujake_info,
    unlocked_vault_info, uosmo_info, AccountToFund, MockEnv,
};

pub mod helpers;

// NOTE: Vault liquidation scenarios spreadsheet:
// https://docs.google.com/spreadsheets/d/1rXa_8eKbtp1wQ0Mm1Rny7QzSLsko9D7UQTtO7NrAssA/edit#gid=2127757089

#[test]
fn liquidatee_must_have_the_request_vault_position() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();
    let leverage_vault = unlocked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .set_params(&[uatom.clone(), uosmo.clone()])
        .vault_configs(&[leverage_vault.clone()])
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
        vec![Liquidate {
            liquidatee_account_id: liquidatee_account_id.clone(),
            debt_coin: uatom.to_coin(10),
            request: LiquidateRequest::Vault {
                request_vault: VaultBase::new(mock.get_vault(&leverage_vault).address),
                position_type: VaultPositionType::UNLOCKED,
            },
        }],
        &[],
    );

    assert_err(
        res,
        ContractError::Std(NotFound {
            kind: "mars_rover::adapters::vault::amount::VaultPositionAmount".to_string(),
        }),
    )
}

#[test]
fn liquidatee_is_not_liquidatable() {
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone()])
        .vault_configs(&[leverage_vault.clone()])
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
                coin: lp_token.to_action_coin(200),
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
        vec![Liquidate {
            liquidatee_account_id: liquidatee_account_id.clone(),
            debt_coin: lp_token.to_coin(10),
            request: LiquidateRequest::Vault {
                request_vault: VaultBase::new(mock.get_vault(&leverage_vault).address),
                position_type: VaultPositionType::UNLOCKED,
            },
        }],
        &[],
    );

    assert_err(
        res,
        ContractError::NotLiquidatable {
            account_id: liquidatee_account_id,
            lqdt_health_factor: "None".to_string(),
        },
    )
}

#[test]
fn liquidator_does_not_have_debt_coin_in_credit_account() {
    let lp_token = lp_token_info();
    let ujake = ujake_info();
    let leverage_vault = unlocked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone(), ujake.clone()])
        .vault_configs(&[leverage_vault.clone()])
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
                coin: lp_token.to_action_coin(200),
            },
            Borrow(ujake.to_coin(175)),
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: ujake.denom.clone(),
        price: Decimal::from_atomics(20u128, 0).unwrap(),
    });

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![Liquidate {
            liquidatee_account_id: liquidatee_account_id.clone(),
            debt_coin: ujake.to_coin(10),
            request: LiquidateRequest::Vault {
                request_vault: VaultBase::new(mock.get_vault(&leverage_vault).address),
                position_type: VaultPositionType::UNLOCKED,
            },
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
fn wrong_position_type_sent_for_unlocked_vault() {
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone()])
        .vault_configs(&[leverage_vault.clone()])
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
                coin: lp_token.to_action_coin(200),
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
        vec![Liquidate {
            liquidatee_account_id: liquidatee_account_id.clone(),
            debt_coin: lp_token.to_coin(10),
            request: LiquidateRequest::Vault {
                request_vault: VaultBase::new(mock.get_vault(&leverage_vault).address),
                position_type: VaultPositionType::LOCKED,
            },
        }],
        &[],
    );

    assert_err(res, ContractError::MismatchedVaultType);

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![Liquidate {
            liquidatee_account_id: liquidatee_account_id.clone(),
            debt_coin: lp_token.to_coin(10),
            request: LiquidateRequest::Vault {
                request_vault: VaultBase::new(mock.get_vault(&leverage_vault).address),
                position_type: VaultPositionType::UNLOCKING,
            },
        }],
        &[],
    );

    assert_err(res, ContractError::MismatchedVaultType)
}

#[test]
fn wrong_position_type_sent_for_locked_vault() {
    let lp_token = lp_token_info();
    let leverage_vault = locked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone()])
        .vault_configs(&[leverage_vault.clone()])
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
                coin: lp_token.to_action_coin(200),
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
        vec![Liquidate {
            liquidatee_account_id: liquidatee_account_id.clone(),
            debt_coin: lp_token.to_coin(10),
            request: LiquidateRequest::Vault {
                request_vault: VaultBase::new(mock.get_vault(&leverage_vault).address),
                position_type: VaultPositionType::UNLOCKED,
            },
        }],
        &[],
    );

    assert_err(res, ContractError::MismatchedVaultType)
}

#[test]
fn liquidate_unlocked_vault() {
    let lp_token = lp_token_info();
    let ujake = ujake_info();
    let leverage_vault = unlocked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let liquidator = Addr::unchecked("liquidator");

    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone(), ujake.clone()])
        .vault_configs(&[leverage_vault.clone()])
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
                coin: lp_token.to_action_coin(200),
            },
            Borrow(ujake.to_coin(175)),
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: ujake.denom.clone(),
        price: Decimal::from_atomics(20u128, 0).unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(ujake.to_coin(10)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: ujake.to_coin(10),
                request: LiquidateRequest::Vault {
                    request_vault: VaultBase::new(mock.get_vault(&leverage_vault).address),
                    position_type: VaultPositionType::UNLOCKED,
                },
            },
        ],
        &[ujake.to_coin(10)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.vaults.len(), 1);
    let vault_balance = position.vaults.first().unwrap().amount.unlocked();
    assert_eq!(vault_balance, Uint128::new(885_000)); // 1M - 115_000

    assert_eq!(position.deposits.len(), 1);
    let jake_balance = get_coin("ujake", &position.deposits);
    assert_eq!(jake_balance.amount, Uint128::new(175));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("ujake", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(166));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.debts.len(), 0);
    let lp = get_coin(&lp_token.denom, &position.deposits);
    assert_eq!(lp.amount, Uint128::new(23));
}

#[test]
fn liquidate_locked_vault() {
    let lp_token = lp_token_info();
    let atom = uatom_info();
    let leverage_vault = locked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let liquidator = Addr::unchecked("liquidator");

    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone(), atom.clone()])
        .vault_configs(&[leverage_vault.clone()])
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
                coin: lp_token.to_action_coin(80),
            },
            Borrow(atom.to_coin(700)),
        ],
        &[lp_token.to_coin(80)],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: atom.denom.clone(),
        price: Decimal::from_atomics(20u128, 0).unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(atom.to_coin(30)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: atom.to_coin(30),
                request: LiquidateRequest::Vault {
                    request_vault: VaultBase::new(mock.get_vault(&leverage_vault).address),
                    position_type: VaultPositionType::LOCKED,
                },
            },
        ],
        &[atom.to_coin(30)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.vaults.len(), 1);
    let vault_amount = position.vaults.first().unwrap().amount.clone();
    // 1M - 825,000 vault tokens liquidated = 175,000
    assert_eq!(vault_amount.locked(), Uint128::new(175_000));
    assert_eq!(vault_amount.unlocking().positions().len(), 0);
    assert_eq!(vault_amount.unlocked(), Uint128::zero());

    assert_eq!(position.deposits.len(), 1);
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(700));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(671)); // 701 - 30

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.debts.len(), 0);
    let lp_balance = get_coin(&lp_token.denom, &position.deposits);
    assert_eq!(lp_balance.amount, Uint128::new(66));
}

#[test]
fn liquidate_unlocking_liquidation_order() {
    let lp_token = lp_token_info();
    let ujake = ujake_info();
    let leverage_vault = locked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let liquidator = Addr::unchecked("liquidator");

    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone(), ujake.clone()])
        .vault_configs(&[leverage_vault.clone()])
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
                coin: lp_token.to_action_coin(200),
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
        price: Decimal::from_atomics(20u128, 0).unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(ujake.to_coin(10)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: ujake.to_coin(10),
                request: LiquidateRequest::Vault {
                    request_vault: VaultBase::new(mock.get_vault(&leverage_vault).address),
                    position_type: VaultPositionType::UNLOCKING,
                },
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

    // Total liquidated:                   24 LP tokens
    //   First bucket drained:                 2 of 2
    //   Second bucket drained:              10 of 10
    //   Third bucket partially liquidated:  11 of 20
    //   Fourth bucket retained:             0 of 168
    assert_eq!(vault_amount.unlocking().positions().len(), 2);
    assert_eq!(vault_amount.unlocking().positions().first().unwrap().coin.amount, Uint128::new(9));
    assert_eq!(vault_amount.unlocking().positions().get(1).unwrap().coin.amount, Uint128::new(168));

    assert_eq!(position.deposits.len(), 1);
    let jake_balance = get_coin("ujake", &position.deposits);
    assert_eq!(jake_balance.amount, Uint128::new(175));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("ujake", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(166));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.debts.len(), 0);
    let lp_balance = get_coin(&lp_token.denom, &position.deposits);
    assert_eq!(lp_balance.amount, Uint128::new(23));
}

// NOTE: liquidation calculation+adjustments are quite complex, full cases in test_liquidate_coin.rs
#[test]
fn liquidation_calculation_adjustment() {
    let lp_token = lp_token_info();
    let ujake = ujake_info();
    let leverage_vault = unlocked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let liquidator = Addr::unchecked("liquidator");

    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone(), ujake.clone()])
        .vault_configs(&[leverage_vault.clone()])
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
                coin: lp_token.to_action_coin(200),
            },
            Borrow(ujake.to_coin(175)),
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: ujake.denom.clone(),
        price: Decimal::from_atomics(20u128, 0).unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(ujake.to_coin(500)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                // Given the request vault balance, this debt payment is too high.
                // It will be adjusted to 85, the max given the request vault value
                debt_coin: ujake.to_coin(500),
                request: LiquidateRequest::Vault {
                    request_vault: VaultBase::new(mock.get_vault(&leverage_vault).address),
                    position_type: VaultPositionType::UNLOCKED,
                },
            },
        ],
        &[ujake.to_coin(500)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.vaults.len(), 1);
    let vault_balance = position.vaults.first().unwrap().amount.unlocked();
    assert_eq!(vault_balance, Uint128::new(15_000)); // Vault position liquidated by 99%

    assert_eq!(position.deposits.len(), 1);
    let jake_balance = get_coin("ujake", &position.deposits);
    assert_eq!(jake_balance.amount, Uint128::new(175));

    assert_eq!(position.debts.len(), 1);
    let jake_debt = get_debt("ujake", &position.debts);
    assert_eq!(jake_debt.amount, Uint128::new(91));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 2);
    let jake_balance = get_coin("ujake", &position.deposits);
    assert_eq!(jake_balance.amount, Uint128::new(415));
    let atom_balance = get_coin(&lp_token.denom, &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(197));
    assert_eq!(position.debts.len(), 0);
}
