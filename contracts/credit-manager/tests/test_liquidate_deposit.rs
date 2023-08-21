use cosmwasm_std::{coins, Addr, Coin, Decimal, OverflowError, OverflowOperation, Uint128};
use mars_mock_oracle::msg::CoinPrice;
use mars_red_bank_types::oracle::ActionKind;
use mars_rover::{
    error::{
        ContractError,
        ContractError::{AboveMaxLTV, LiquidationNotProfitable, NotLiquidatable},
    },
    msg::execute::{
        Action::{Borrow, Deposit, EnterVault, Liquidate, Withdraw},
        LiquidateRequest,
    },
};
use mars_rover_health_types::AccountKind;

use crate::helpers::{
    assert_err, get_coin, get_debt, lp_token_info, uatom_info, ujake_info, unlocked_vault_info,
    uosmo_info, AccountToFund, MockEnv,
};

pub mod helpers;

// Reference figures behind various scenarios
// https://docs.google.com/spreadsheets/d/14Dk0L2oqI4gOKQZqe12TyjE4ZbVsJMViN1h1B4sJaQs/edit#gid=884610559

#[test]
fn can_only_liquidate_unhealthy_accounts() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .set_params(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: coins(300, uosmo_info.denom.clone()),
        })
        .build()
        .unwrap();
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![Deposit(uosmo_info.to_coin(300)), Borrow(uatom_info.to_coin(50))],
        &[Coin::new(300, uosmo_info.clone().denom)],
    )
    .unwrap();

    let health =
        mock.query_health(&liquidatee_account_id, AccountKind::Default, ActionKind::Liquidation);
    assert!(!health.liquidatable);

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![Liquidate {
            liquidatee_account_id: liquidatee_account_id.clone(),
            debt_coin: uatom_info.to_coin(10),
            request: LiquidateRequest::Deposit(uosmo_info.denom),
        }],
        &[],
    );

    assert_err(
        res,
        NotLiquidatable {
            account_id: liquidatee_account_id,
            lqdt_health_factor: "2.019607843137254901".to_string(),
        },
    )
}

#[test]
fn vault_positions_contribute_to_health() {
    let atom_info = uatom_info();
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone(), atom_info.clone()])
        .vault_configs(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![lp_token.to_coin(500)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(lp_token.to_coin(220)),
            EnterVault {
                vault,
                coin: lp_token.to_action_coin(200),
            },
            Borrow(atom_info.to_coin(14)),
        ],
        &[lp_token.to_coin(220)],
    )
    .unwrap();

    let health =
        mock.query_health(&liquidatee_account_id, AccountKind::Default, ActionKind::Liquidation);
    assert!(!health.liquidatable);

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![Liquidate {
            liquidatee_account_id: liquidatee_account_id.clone(),
            debt_coin: atom_info.to_coin(10),
            request: LiquidateRequest::Deposit(atom_info.denom),
        }],
        &[],
    );

    assert_err(
        res,
        NotLiquidatable {
            account_id: liquidatee_account_id,
            lqdt_health_factor: "101.733333333333333333".to_string(),
        },
    )
}

#[test]
fn liquidatee_does_not_have_requested_asset() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let ujake_info = ujake_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .set_params(&[uosmo_info.clone(), uatom_info.clone(), ujake_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: coins(300, uosmo_info.denom.clone()),
        })
        .build()
        .unwrap();
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![Deposit(uosmo_info.to_coin(300)), Borrow(uatom_info.to_coin(105))],
        &[Coin::new(300, uosmo_info.denom)],
    )
    .unwrap();

    let health =
        mock.query_health(&liquidatee_account_id, AccountKind::Default, ActionKind::Liquidation);
    assert!(!health.liquidatable);

    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(20u128, 0).unwrap(),
    });

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Borrow(uatom_info.to_coin(50)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(10),
                request: LiquidateRequest::Deposit(ujake_info.denom.clone()),
            },
        ],
        &[],
    );

    assert_err(res, ContractError::CoinNotAvailable(ujake_info.denom))
}

#[test]
fn liquidatee_does_not_have_debt_coin() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let ujake_info = ujake_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let random_user = Addr::unchecked("random_user");
    let mut mock = MockEnv::new()
        .set_params(&[uosmo_info.clone(), uatom_info.clone(), ujake_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: coins(300, uosmo_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: random_user.clone(),
            funds: coins(300, uosmo_info.denom.clone()),
        })
        .build()
        .unwrap();
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![Deposit(uosmo_info.to_coin(300)), Borrow(uatom_info.to_coin(105))],
        &[Coin::new(300, uosmo_info.denom.clone())],
    )
    .unwrap();

    let health =
        mock.query_health(&liquidatee_account_id, AccountKind::Default, ActionKind::Liquidation);
    assert!(!health.liquidatable);

    // Seeding a jakecoin borrow
    let random_user_token = mock.create_credit_account(&random_user).unwrap();
    mock.update_credit_account(
        &random_user_token,
        &random_user,
        vec![Deposit(uosmo_info.to_coin(300)), Borrow(ujake_info.to_coin(10))],
        &[Coin::new(300, uosmo_info.denom)],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(20u128, 0).unwrap(),
    });

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Borrow(uatom_info.to_coin(50)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: ujake_info.to_coin(10),
                request: LiquidateRequest::Deposit(uatom_info.denom),
            },
        ],
        &[],
    );

    assert_err(res, ContractError::NoDebt)
}

#[test]
fn liquidator_does_not_have_enough_to_pay_debt() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .set_params(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: coins(300, uosmo_info.denom.clone()),
        })
        .build()
        .unwrap();
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![Deposit(uosmo_info.to_coin(300)), Borrow(uatom_info.to_coin(100))],
        &[Coin::new(300, uosmo_info.clone().denom)],
    )
    .unwrap();

    let health =
        mock.query_health(&liquidatee_account_id, AccountKind::Default, ActionKind::Liquidation);
    assert!(!health.liquidatable);

    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(10u128, 0).unwrap(),
    });

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![Liquidate {
            liquidatee_account_id: liquidatee_account_id.clone(),
            debt_coin: uatom_info.to_coin(10),
            request: LiquidateRequest::Deposit(uosmo_info.denom),
        }],
        &[],
    );

    assert_err(
        res,
        ContractError::Overflow(OverflowError {
            operation: OverflowOperation::Sub,
            operand1: "0".to_string(),
            operand2: "7".to_string(),
        }),
    )
}

#[test]
fn liquidator_left_in_unhealthy_state() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .set_params(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: coins(300, uosmo_info.denom.clone()),
        })
        .build()
        .unwrap();
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![Deposit(uosmo_info.to_coin(300)), Borrow(uatom_info.to_coin(100))],
        &[Coin::new(300, uosmo_info.clone().denom)],
    )
    .unwrap();

    let health =
        mock.query_health(&liquidatee_account_id, AccountKind::Default, ActionKind::Liquidation);
    assert!(!health.liquidatable);

    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(10u128, 0).unwrap(),
    });

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Borrow(uatom_info.to_coin(1000)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(10),
                request: LiquidateRequest::Deposit(uosmo_info.denom),
            },
        ],
        &[],
    );

    assert_err(
        res,
        AboveMaxLTV {
            account_id: liquidator_account_id,
            max_ltv_health_factor: "0.863136863136863136".to_string(),
        },
    )
}

#[test]
fn liquidation_not_profitable_after_calculations() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let ujake_info = ujake_info();
    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .set_params(&[uosmo_info.clone(), uatom_info.clone(), ujake_info.clone()])
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

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uosmo_info.to_coin(300)),
            Borrow(uatom_info.to_coin(100)),
            Borrow(ujake_info.to_coin(25)),
        ],
        &[Coin::new(300, uosmo_info.denom.clone())],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: ujake_info.denom,
        price: Decimal::from_atomics(100u128, 0).unwrap(),
    });

    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: uosmo_info.denom.clone(),
        price: Decimal::from_atomics(2u128, 0).unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(10)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(5),
                request: LiquidateRequest::Deposit(uosmo_info.denom.clone()),
            },
        ],
        &[uatom_info.to_coin(10)],
    );

    assert_err(
        res,
        LiquidationNotProfitable {
            debt_coin: uatom_info.to_coin(5),
            request_coin: uosmo_info.to_coin(2),
        },
    )
}

#[test]
fn target_health_factor_reached_after_max_debt_repayed() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");
    let thf = Decimal::from_atomics(12u128, 1).unwrap();
    let mut mock = MockEnv::new()
        .target_health_factor(thf)
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
            Withdraw(uatom_info.to_action_coin(400)),
        ],
        &[Coin::new(3000, uosmo_info.denom.clone())],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(128u128, 2).unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(561)), // MDR = 505, refund 56
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(561),
                request: LiquidateRequest::Deposit(uosmo_info.denom),
            },
        ],
        &[uatom_info.to_coin(561)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.deposits.len(), 2);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(368));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(600));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(496));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_eq!(position.debts.len(), 0);
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(56));
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(2631));

    // Assert rewards-collector's new position
    let rewards_collector_acc_id = mock.query_rewards_collector_account();
    let position = mock.query_positions(&rewards_collector_acc_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.debts.len(), 0);
    let atom_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(1));

    // Assert HF for liquidatee
    let account_kind = mock.query_account_kind(&liquidatee_account_id);
    let health = mock.query_health(&liquidatee_account_id, account_kind, ActionKind::Liquidation);
    // it should be 1.2, but because of roundings it is hard to achieve an exact number
    let health_diff = health.liquidation_health_factor.unwrap().abs_diff(thf);
    assert!(health_diff < Decimal::from_atomics(1u128, 2u32).unwrap());
}

#[test]
fn debt_amount_adjusted_to_total_debt_for_denom() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let ujake_info = ujake_info();
    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .target_health_factor(Decimal::from_atomics(12u128, 1).unwrap())
        .set_params(&[uosmo_info.clone(), uatom_info.clone(), ujake_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: coins(3000, uosmo_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: coins(3000, ujake_info.denom.clone()),
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
            Borrow(ujake_info.to_coin(100)),
        ],
        &[Coin::new(3000, uosmo_info.denom.clone())],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: uatom_info.denom,
        price: Decimal::from_atomics(5u128, 0).unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(ujake_info.to_coin(101)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: ujake_info.to_coin(101),
                request: LiquidateRequest::Deposit(uosmo_info.denom),
            },
        ],
        &[ujake_info.to_coin(101)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.deposits.len(), 3);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(2028));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(1000));
    let jake_balance = get_coin("ujake", &position.deposits);
    assert_eq!(jake_balance.amount, Uint128::new(100));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(1001));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.debts.len(), 0);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(971));

    // Assert rewards-collector's new position
    let rewards_collector_acc_id = mock.query_rewards_collector_account();
    let position = mock.query_positions(&rewards_collector_acc_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.debts.len(), 0);
    let atom_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(1));

    // Liq HF should improve
    let account_kind = mock.query_account_kind(&liquidatee_account_id);
    let health = mock.query_health(&liquidatee_account_id, account_kind, ActionKind::Liquidation);
    assert!(!health.liquidatable);
}

#[test]
fn debt_amount_adjusted_to_max_allowed_by_request_coin() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
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
        vec![Deposit(uosmo_info.to_coin(3000)), Borrow(uatom_info.to_coin(1000))],
        &[Coin::new(3000, uosmo_info.denom.clone())],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(6u128, 0).unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(144)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(122),
                request: LiquidateRequest::Deposit(uosmo_info.denom),
            },
        ],
        &[uatom_info.to_coin(144)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.deposits.len(), 2);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(24));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(1000));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(879));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_eq!(position.debts.len(), 0);
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(22));
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(2975));

    // Assert rewards-collector's new position
    let rewards_collector_acc_id = mock.query_rewards_collector_account();
    let position = mock.query_positions(&rewards_collector_acc_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.debts.len(), 0);
    let atom_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(1));

    // Liq HF should improve
    let account_kind = mock.query_account_kind(&liquidatee_account_id);
    let health = mock.query_health(&liquidatee_account_id, account_kind, ActionKind::Liquidation);
    assert!(!health.liquidatable);
}

#[test]
fn debt_amount_no_adjustment() {
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
        vec![Deposit(uosmo_info.to_coin(3000)), Borrow(uatom_info.to_coin(1000))],
        &[Coin::new(3000, uosmo_info.denom.clone())],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(59u128, 1).unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(100)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(100),
                request: LiquidateRequest::Deposit(uosmo_info.denom),
            },
        ],
        &[uatom_info.to_coin(100)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.deposits.len(), 2);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(608));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(1000));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(901));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.debts.len(), 0);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(2391));

    // Assert rewards-collector's new position
    let rewards_collector_acc_id = mock.query_rewards_collector_account();
    let position = mock.query_positions(&rewards_collector_acc_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.debts.len(), 0);
    let atom_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(1));

    // Liq HF should improve
    let account_kind = mock.query_account_kind(&liquidatee_account_id);
    let health = mock.query_health(&liquidatee_account_id, account_kind, ActionKind::Liquidation);
    assert!(!health.liquidatable);
}

#[test]
fn improve_hf_but_acc_unhealthy() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let ujake_info = ujake_info();
    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .target_health_factor(Decimal::from_atomics(12u128, 1).unwrap())
        .set_params(&[uosmo_info.clone(), uatom_info.clone(), ujake_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: coins(4000, uosmo_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: coins(4000, ujake_info.denom.clone()),
        })
        .build()
        .unwrap();
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uosmo_info.to_coin(4000)),
            Borrow(uatom_info.to_coin(1000)),
            Borrow(ujake_info.to_coin(430)),
        ],
        &[Coin::new(4000, uosmo_info.denom.clone())],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: uatom_info.denom,
        price: Decimal::from_atomics(10u128, 0).unwrap(),
    });

    let account_kind = mock.query_account_kind(&liquidatee_account_id);
    let prev_health =
        mock.query_health(&liquidatee_account_id, account_kind.clone(), ActionKind::Liquidation);

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(ujake_info.to_coin(138)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: ujake_info.to_coin(120),
                request: LiquidateRequest::Deposit(uosmo_info.denom),
            },
        ],
        &[ujake_info.to_coin(138)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.deposits.len(), 3);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(2768));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(1000));
    let jake_balance = get_coin("ujake", &position.deposits);
    assert_eq!(jake_balance.amount, Uint128::new(430));

    assert_eq!(position.debts.len(), 2);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(1001));
    let jake_debt = get_debt("ujake", &position.debts);
    assert_eq!(jake_debt.amount, Uint128::new(311));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_eq!(position.debts.len(), 0);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(1229));
    let jake_balance = get_coin("ujake", &position.deposits);
    assert_eq!(jake_balance.amount, Uint128::new(18));

    // Assert rewards-collector's new position
    let rewards_collector_acc_id = mock.query_rewards_collector_account();
    let position = mock.query_positions(&rewards_collector_acc_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.debts.len(), 0);
    let atom_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(3));

    // Liq HF should improve
    let health = mock.query_health(&liquidatee_account_id, account_kind, ActionKind::Liquidation);
    assert!(health.liquidatable);
    assert!(
        prev_health.liquidation_health_factor.unwrap() < health.liquidation_health_factor.unwrap()
    );
}
