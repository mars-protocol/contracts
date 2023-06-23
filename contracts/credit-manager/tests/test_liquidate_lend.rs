use cosmwasm_std::{coins, Addr, Decimal, Uint128};
use mars_mock_oracle::msg::CoinPrice;
use mars_rover::{
    error::{ContractError, ContractError::NotLiquidatable},
    msg::execute::{
        Action::{Borrow, Deposit, Lend, Liquidate, Reclaim},
        ActionAmount, ActionCoin, LiquidateRequest,
    },
};
use mars_rover_health_types::AccountKind;

use crate::helpers::{
    assert_err, get_coin, get_debt, get_lent, uatom_info, ujake_info, uosmo_info, AccountToFund,
    MockEnv,
};

pub mod helpers;

// Reference figures behind various scenarios
// https://docs.google.com/spreadsheets/d/1H7Ajghsee2l7_litG7EWoM-kkVQOh4dbHa8WSV-Y6Jg/edit#gid=1331087474

#[test]
fn lent_positions_contribute_to_health() {
    let uatom_info = uatom_info();
    let uosmo_info = uosmo_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .set_params(&[uatom_info.clone(), uosmo_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![uatom_info.to_coin(500), uosmo_info.to_coin(500)],
        })
        .build()
        .unwrap();

    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![Deposit(uatom_info.to_coin(100)), Borrow(uosmo_info.to_coin(40))],
        &[uatom_info.to_coin(100)],
    )
    .unwrap();

    let health_1 = mock.query_health(&liquidatee_account_id, AccountKind::Default);
    assert!(!health_1.liquidatable);

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![Lend(uatom_info.to_coin(50))],
        &[],
    )
    .unwrap();

    // Collateral should be the same after Lend
    let health_2 = mock.query_health(&liquidatee_account_id, AccountKind::Default);
    assert!(!health_2.liquidatable);
    // health_2.total_collateral_value bigger (+1) because of simulated yield
    assert_eq!(health_1.total_collateral_value, health_2.total_collateral_value - Uint128::one());
    assert_eq!(health_1.max_ltv_adjusted_collateral, health_2.max_ltv_adjusted_collateral);
    assert_eq!(
        health_1.liquidation_threshold_adjusted_collateral,
        health_2.liquidation_threshold_adjusted_collateral
    );

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![Liquidate {
            liquidatee_account_id: liquidatee_account_id.clone(),
            debt_coin: uosmo_info.to_coin(10),
            request: LiquidateRequest::Lend(uatom_info.denom),
        }],
        &[],
    );

    assert_err(
        res,
        NotLiquidatable {
            account_id: liquidatee_account_id,
            lqdt_health_factor: "8.818181818181818181".to_string(),
        },
    )
}

#[test]
fn liquidatee_does_not_have_requested_lent_coin() {
    let uatom_info = uatom_info();
    let uosmo_info = uosmo_info();
    let ujake_info = ujake_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let liquidator = Addr::unchecked("liquidator");

    let mut mock = MockEnv::new()
        .set_params(&[uatom_info.clone(), uosmo_info.clone(), ujake_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![uatom_info.to_coin(500)],
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: vec![uosmo_info.to_coin(500)],
        })
        .build()
        .unwrap();

    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uatom_info.to_coin(100)),
            Lend(uatom_info.to_coin(50)),
            Borrow(uosmo_info.to_coin(100)),
        ],
        &[uatom_info.to_coin(100)],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: uosmo_info.denom.clone(),
        price: Decimal::from_atomics(20u128, 0).unwrap(),
    });

    let health = mock.query_health(&liquidatee_account_id, AccountKind::Default);
    assert!(health.liquidatable);

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uosmo_info.to_coin(10)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uosmo_info.to_coin(10),
                request: LiquidateRequest::Lend(ujake_info.denom),
            },
        ],
        &[uosmo_info.to_coin(10)],
    );

    assert_err(res, ContractError::NoneLent);
}

#[test]
fn lent_position_partially_liquidated() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();

    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");

    let mut mock = MockEnv::new()
        .target_health_factor(Decimal::from_atomics(12u128, 1).unwrap())
        .set_params(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: coins(2000, uosmo_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: coins(2000, uatom_info.denom.clone()),
        })
        .build()
        .unwrap();

    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uosmo_info.to_coin(1050)),
            Borrow(uatom_info.to_coin(1000)),
            Lend(uosmo_info.to_coin(450)),
        ],
        &[uosmo_info.to_coin(1050)],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(22u128, 1).unwrap(),
    });

    let health = mock.query_health(&liquidatee_account_id, AccountKind::Default);
    assert!(health.liquidatable);
    assert_eq!(health.total_collateral_value, Uint128::new(2462u128));
    assert_eq!(health.total_debt_value, Uint128::new(2203u128));

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(45)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(45),
                request: LiquidateRequest::Lend(uosmo_info.denom),
            },
        ],
        &[uatom_info.to_coin(45)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.deposits.len(), 2);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(600));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(1000));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(956));

    assert_eq!(position.lends.len(), 1);
    let osmo_lent = get_lent("uosmo", &position.lends);
    assert_eq!(osmo_lent.amount, Uint128::new(39));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 0);
    assert_eq!(position.debts.len(), 0);

    assert_eq!(position.lends.len(), 1);
    let osmo_lent = get_lent("uosmo", &position.lends);
    assert_eq!(osmo_lent.amount, Uint128::new(403));

    // Assert rewards-collector's new position
    let rewards_collector_acc_id = mock.query_rewards_collector_account();
    let position = mock.query_positions(&rewards_collector_acc_id);
    assert_eq!(position.deposits.len(), 0);
    assert_eq!(position.debts.len(), 0);

    assert_eq!(position.lends.len(), 1);
    let rc_osmo_lent = get_lent("uosmo", &position.lends);
    assert_eq!(rc_osmo_lent.amount, Uint128::new(8));

    // Liq HF should improve
    let account_kind = mock.query_account_kind(&liquidatee_account_id);
    let health = mock.query_health(&liquidatee_account_id, account_kind);
    assert!(!health.liquidatable);
}

#[test]
fn lent_position_fully_liquidated() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();

    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");

    let mut mock = MockEnv::new()
        .target_health_factor(Decimal::from_atomics(12u128, 1).unwrap())
        .set_params(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: coins(300, uosmo_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: coins(300, uatom_info.denom.clone()),
        })
        .build()
        .unwrap();

    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.price_change(CoinPrice {
        denom: uosmo_info.denom.clone(),
        price: Decimal::from_atomics(10u128, 1).unwrap(),
    });

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uosmo_info.to_coin(300)),
            Borrow(uatom_info.to_coin(500)),
            Lend(uosmo_info.to_coin(109)),
        ],
        &[uosmo_info.to_coin(300)],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(50u128, 1).unwrap(),
    });

    let prev_health = mock.query_health(&liquidatee_account_id, AccountKind::Default);
    assert!(prev_health.liquidatable);
    assert_eq!(prev_health.total_collateral_value, Uint128::new(2801u128));
    assert_eq!(prev_health.total_debt_value, Uint128::new(2505u128));

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(32)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(32),
                request: LiquidateRequest::Lend(uosmo_info.denom),
            },
        ],
        &[uatom_info.to_coin(32)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.deposits.len(), 2);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(191));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(500));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(480));

    // FIXME: dust because of roundings, is it possible to avoid it?
    assert_eq!(position.lends.len(), 1);
    let osmo_balance = get_lent("uosmo", &position.lends);
    assert_eq!(osmo_balance.amount, Uint128::new(1));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 1);
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(11));

    assert_eq!(position.debts.len(), 0);

    assert_eq!(position.lends.len(), 1);
    let osmo_lent = get_lent("uosmo", &position.lends);
    assert_eq!(osmo_lent.amount, Uint128::new(106));

    // Assert rewards-collector's new position
    let rewards_collector_acc_id = mock.query_rewards_collector_account();
    let position = mock.query_positions(&rewards_collector_acc_id);
    assert_eq!(position.deposits.len(), 0);
    assert_eq!(position.debts.len(), 0);

    assert_eq!(position.lends.len(), 1);
    let rc_osmo_lent = get_lent("uosmo", &position.lends);
    // FIXME: excel shows 2, simulated interest rate influence?
    assert_eq!(rc_osmo_lent.amount, Uint128::new(1));

    // Liq HF should improve
    let account_kind = mock.query_account_kind(&liquidatee_account_id);
    let health = mock.query_health(&liquidatee_account_id, account_kind);
    assert!(health.liquidatable);
    assert!(
        prev_health.liquidation_health_factor.unwrap() < health.liquidation_health_factor.unwrap()
    );
}

#[test]
fn liquidate_with_reclaiming() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();

    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");

    let mut mock = MockEnv::new()
        .target_health_factor(Decimal::from_atomics(12u128, 1).unwrap())
        .set_params(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: coins(3000, uosmo_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: coins(3000, uatom_info.denom.clone()),
        })
        .build()
        .unwrap();

    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uosmo_info.to_coin(3000)),
            Borrow(uatom_info.to_coin(1000)),
            Lend(uosmo_info.to_coin(1500)),
        ],
        &[uosmo_info.to_coin(3000)],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(82u128, 1).unwrap(),
    });

    let prev_health = mock.query_health(&liquidatee_account_id, AccountKind::Default);
    assert!(prev_health.liquidatable);
    assert_eq!(prev_health.total_collateral_value, Uint128::new(8950u128));
    assert_eq!(prev_health.total_debt_value, Uint128::new(8209u128));

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(100)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(100),
                request: LiquidateRequest::Lend(uosmo_info.denom.clone()),
            },
            Reclaim(ActionCoin {
                denom: uosmo_info.denom,
                amount: ActionAmount::AccountBalance,
            }),
        ],
        &[uatom_info.to_coin(100)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.deposits.len(), 2);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(1500));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(1000));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(960));

    assert_eq!(position.lends.len(), 1);
    let osmo_lent = get_lent("uosmo", &position.lends);
    // FIXME: excel shows 37, simulated interest rate influence?
    assert_eq!(osmo_lent.amount, Uint128::new(36));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 2);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(1435));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(59));

    assert_eq!(position.debts.len(), 0);

    assert_eq!(position.lends.len(), 0);

    // Assert rewards-collector's new position
    let rewards_collector_acc_id = mock.query_rewards_collector_account();
    let position = mock.query_positions(&rewards_collector_acc_id);
    assert_eq!(position.deposits.len(), 0);
    assert_eq!(position.debts.len(), 0);

    assert_eq!(position.lends.len(), 1);
    let rc_osmo_lent = get_lent("uosmo", &position.lends);
    // FIXME: excel shows 28, simulated interest rate influence?
    assert_eq!(rc_osmo_lent.amount, Uint128::new(27));

    // Liq HF should improve
    let account_kind = mock.query_account_kind(&liquidatee_account_id);
    let health = mock.query_health(&liquidatee_account_id, account_kind);
    assert!(health.liquidatable);
    assert!(
        prev_health.liquidation_health_factor.unwrap() < health.liquidation_health_factor.unwrap()
    );
}
