use cosmwasm_std::{coins, Addr, Coin, Decimal, OverflowError, OverflowOperation, Uint128};
use mars_mock_oracle::msg::CoinPrice;
use mars_rover::{
    error::{
        ContractError,
        ContractError::{AboveMaxLTV, LiquidationNotProfitable, NotLiquidatable},
    },
    msg::execute::{
        Action::{Borrow, Deposit, EnterVault, Liquidate},
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
// https://docs.google.com/spreadsheets/d/1_Bs1Fc1RLf5IARvaXZ0QjigoMWSJQhhrRUtQ8uyoLdI/edit?pli=1#gid=1857897311

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

    let health = mock.query_health(&liquidatee_account_id, AccountKind::Default);
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

    let health = mock.query_health(&liquidatee_account_id, AccountKind::Default);
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

    let health = mock.query_health(&liquidatee_account_id, AccountKind::Default);
    assert!(!health.liquidatable);

    mock.price_change(CoinPrice {
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

    let health = mock.query_health(&liquidatee_account_id, AccountKind::Default);
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

    let health = mock.query_health(&liquidatee_account_id, AccountKind::Default);
    assert!(!health.liquidatable);

    mock.price_change(CoinPrice {
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(20u128, 0).unwrap(),
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
            operand2: "3".to_string(),
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

    let health = mock.query_health(&liquidatee_account_id, AccountKind::Default);
    assert!(!health.liquidatable);

    mock.price_change(CoinPrice {
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(20u128, 0).unwrap(),
    });

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Borrow(uatom_info.to_coin(10)),
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
            max_ltv_health_factor: "0.727272727272727272".to_string(),
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
        denom: ujake_info.denom,
        price: Decimal::from_atomics(100u128, 0).unwrap(),
    });

    mock.price_change(CoinPrice {
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
fn debt_amount_adjusted_to_close_factor_max() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .max_close_factor(Decimal::from_atomics(1u128, 1).unwrap())
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

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![Deposit(uosmo_info.to_coin(300)), Borrow(uatom_info.to_coin(100))],
        &[Coin::new(300, uosmo_info.denom.clone())],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(6u128, 0).unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(50)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(50),
                request: LiquidateRequest::Deposit(uosmo_info.denom),
            },
        ],
        &[uatom_info.to_coin(50)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.deposits.len(), 2);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(36));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(100));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(91));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_eq!(position.debts.len(), 0);
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(40));
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(264));
}

#[test]
fn debt_amount_adjusted_to_total_debt_for_denom() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let ujake_info = ujake_info();
    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .max_close_factor(Decimal::from_atomics(1u128, 1).unwrap())
        .set_params(&[uosmo_info.clone(), uatom_info.clone(), ujake_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: coins(300, uosmo_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: coins(300, ujake_info.denom.clone()),
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
            Borrow(ujake_info.to_coin(10)),
        ],
        &[Coin::new(300, uosmo_info.denom.clone())],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: uatom_info.denom,
        price: Decimal::from_atomics(20u128, 0).unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(ujake_info.to_coin(50)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: ujake_info.to_coin(50),
                request: LiquidateRequest::Deposit(uosmo_info.denom),
            },
        ],
        &[ujake_info.to_coin(50)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.deposits.len(), 3);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(184));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(100));
    let jake_balance = get_coin("ujake", &position.deposits);
    assert_eq!(jake_balance.amount, Uint128::new(10));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(101));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_eq!(position.debts.len(), 0);
    let jake_balance = get_coin("ujake", &position.deposits);
    assert_eq!(jake_balance.amount, Uint128::new(39));
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(116));
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
        vec![Deposit(uosmo_info.to_coin(300)), Borrow(uatom_info.to_coin(100))],
        &[Coin::new(300, uosmo_info.denom.clone())],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(20u128, 0).unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(50)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(50),
                request: LiquidateRequest::Deposit(uosmo_info.denom),
            },
        ],
        &[uatom_info.to_coin(50)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.deposits.len(), 2);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(36));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(100));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(98));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_eq!(position.debts.len(), 0);
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(47));
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(264));
}

#[test]
fn debt_amount_no_adjustment() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .max_close_factor(Decimal::from_atomics(1u128, 1).unwrap())
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

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![Deposit(uosmo_info.to_coin(300)), Borrow(uatom_info.to_coin(100))],
        &[Coin::new(300, uosmo_info.denom.clone())],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(55u128, 1).unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(10)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(10),
                request: LiquidateRequest::Deposit(uosmo_info.denom),
            },
        ],
        &[uatom_info.to_coin(10)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.deposits.len(), 2);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(60));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(100));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(91));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.debts.len(), 0);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(240));
}

#[test]
fn liquidate_with_no_deposited_funds() {}
