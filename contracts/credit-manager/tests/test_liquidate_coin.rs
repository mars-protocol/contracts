use cosmwasm_std::{coins, Addr, Coin, Decimal, OverflowError, OverflowOperation, Uint128};

use mock_oracle::msg::CoinPrice;
use rover::error::ContractError;
use rover::error::ContractError::{AboveMaxLTV, NotLiquidatable};
use rover::msg::execute::Action::{Borrow, Deposit, EnterVault, LiquidateCoin};
use rover::traits::IntoDecimal;

use crate::helpers::{
    assert_err, get_coin, get_debt, lp_token_info, uatom_info, ujake_info, unlocked_vault_info,
    uosmo_info, AccountToFund, MockEnv,
};

pub mod helpers;

// Reference figures behind various scenarios
// https://docs.google.com/spreadsheets/d/1_Bs1Fc1RLf5IARvaXZ0QjigoMWSJQhhrRUtQ8uyoLdI/edit?pli=1#gid=1857897311

#[test]
fn test_can_only_liquidate_unhealthy_accounts() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uosmo_info.clone(), uatom_info.clone()])
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
        vec![
            Deposit(uosmo_info.to_coin(300)),
            Borrow(uatom_info.to_coin(50)),
        ],
        &[Coin::new(300, uosmo_info.clone().denom)],
    )
    .unwrap();

    let health = mock.query_health(&liquidatee_account_id);
    assert!(!health.liquidatable);

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![LiquidateCoin {
            liquidatee_account_id: liquidatee_account_id.clone(),
            debt_coin: uatom_info.to_coin(10),
            request_coin_denom: uosmo_info.denom,
        }],
        &[],
    );

    assert_err(
        res,
        NotLiquidatable {
            account_id: liquidatee_account_id,
            lqdt_health_factor: "2.029411764705882352".to_string(),
        },
    )
}

#[test]
fn test_vault_positions_contribute_to_health() {
    let atom_info = uatom_info();
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone(), atom_info.clone()])
        .allowed_vaults(&[leverage_vault.clone()])
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
                denom: lp_token.denom.clone(),
                amount: Some(Uint128::new(200)),
            },
            Borrow(atom_info.to_coin(14)),
        ],
        &[lp_token.to_coin(220)],
    )
    .unwrap();

    let health = mock.query_health(&liquidatee_account_id);
    assert!(!health.liquidatable);

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![LiquidateCoin {
            liquidatee_account_id: liquidatee_account_id.clone(),
            debt_coin: atom_info.to_coin(10),
            request_coin_denom: atom_info.denom,
        }],
        &[],
    );

    assert_err(
        res,
        NotLiquidatable {
            account_id: liquidatee_account_id,
            lqdt_health_factor: "101.94976".to_string(),
        },
    )
}

#[test]
fn test_liquidatee_does_not_have_requested_asset() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let ujake_info = ujake_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uosmo_info.clone(), uatom_info.clone(), ujake_info.clone()])
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
        vec![
            Deposit(uosmo_info.to_coin(300)),
            Borrow(uatom_info.to_coin(105)),
        ],
        &[Coin::new(300, uosmo_info.denom)],
    )
    .unwrap();

    let health = mock.query_health(&liquidatee_account_id);
    assert!(!health.liquidatable);

    mock.price_change(CoinPrice {
        denom: uatom_info.denom.clone(),
        price: 20.to_dec().unwrap(),
    });

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Borrow(uatom_info.to_coin(50)),
            LiquidateCoin {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(10),
                request_coin_denom: ujake_info.denom.clone(),
            },
        ],
        &[],
    );

    assert_err(res, ContractError::CoinNotAvailable(ujake_info.denom))
}

#[test]
fn test_liquidatee_does_not_have_debt_coin() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let ujake_info = ujake_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let random_user = Addr::unchecked("random_user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uosmo_info.clone(), uatom_info.clone(), ujake_info.clone()])
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
        vec![
            Deposit(uosmo_info.to_coin(300)),
            Borrow(uatom_info.to_coin(105)),
        ],
        &[Coin::new(300, uosmo_info.denom.clone())],
    )
    .unwrap();

    let health = mock.query_health(&liquidatee_account_id);
    assert!(!health.liquidatable);

    // Seeding a jakecoin borrow
    let random_user_token = mock.create_credit_account(&random_user).unwrap();
    mock.update_credit_account(
        &random_user_token,
        &random_user,
        vec![
            Deposit(uosmo_info.to_coin(300)),
            Borrow(ujake_info.to_coin(10)),
        ],
        &[Coin::new(300, uosmo_info.denom)],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: uatom_info.denom.clone(),
        price: 20.to_dec().unwrap(),
    });

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Borrow(uatom_info.to_coin(50)),
            LiquidateCoin {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: ujake_info.to_coin(10),
                request_coin_denom: uatom_info.denom,
            },
        ],
        &[],
    );

    assert_err(res, ContractError::NoDebt)
}

#[test]
fn test_liquidator_does_not_have_enough_to_pay_debt() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uosmo_info.clone(), uatom_info.clone()])
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
        vec![
            Deposit(uosmo_info.to_coin(300)),
            Borrow(uatom_info.to_coin(100)),
        ],
        &[Coin::new(300, uosmo_info.clone().denom)],
    )
    .unwrap();

    let health = mock.query_health(&liquidatee_account_id);
    assert!(!health.liquidatable);

    mock.price_change(CoinPrice {
        denom: uatom_info.denom.clone(),
        price: 20.to_dec().unwrap(),
    });

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![LiquidateCoin {
            liquidatee_account_id: liquidatee_account_id.clone(),
            debt_coin: uatom_info.to_coin(10),
            request_coin_denom: uosmo_info.denom,
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
fn test_liquidator_left_in_unhealthy_state() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uosmo_info.clone(), uatom_info.clone()])
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
        vec![
            Deposit(uosmo_info.to_coin(300)),
            Borrow(uatom_info.to_coin(100)),
        ],
        &[Coin::new(300, uosmo_info.clone().denom)],
    )
    .unwrap();

    let health = mock.query_health(&liquidatee_account_id);
    assert!(!health.liquidatable);

    mock.price_change(CoinPrice {
        denom: uatom_info.denom.clone(),
        price: 20.to_dec().unwrap(),
    });

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Borrow(uatom_info.to_coin(10)),
            LiquidateCoin {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(10),
                request_coin_denom: uosmo_info.denom,
            },
        ],
        &[],
    );

    assert_err(
        res,
        AboveMaxLTV {
            account_id: liquidator_account_id,
            max_ltv_health_factor: "0.795375".to_string(),
        },
    )
}

#[test]
fn test_debt_amount_adjusted_to_close_factor_max() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .max_close_factor(Decimal::from_atomics(1u128, 1).unwrap())
        .allowed_coins(&[uosmo_info.clone(), uatom_info.clone()])
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
        ],
        &[Coin::new(300, uosmo_info.denom.clone())],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: uatom_info.denom.clone(),
        price: 6.to_dec().unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(50)),
            LiquidateCoin {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(50),
                request_coin_denom: uosmo_info.denom,
            },
        ],
        &[uatom_info.to_coin(50)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.coins.len(), 2);
    let osmo_balance = get_coin("uosmo", &position.coins);
    assert_eq!(osmo_balance.amount, Uint128::new(47));
    let atom_balance = get_coin("uatom", &position.coins);
    assert_eq!(atom_balance.amount, Uint128::new(100));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(91));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.coins.len(), 2);
    assert_eq!(position.debts.len(), 0);
    let atom_balance = get_coin("uatom", &position.coins);
    assert_eq!(atom_balance.amount, Uint128::new(40));
    let osmo_balance = get_coin("uosmo", &position.coins);
    assert_eq!(osmo_balance.amount, Uint128::new(253));
}

#[test]
fn test_debt_amount_adjusted_to_total_debt_for_denom() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let ujake_info = ujake_info();
    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .max_close_factor(Decimal::from_atomics(1u128, 1).unwrap())
        .allowed_coins(&[uosmo_info.clone(), uatom_info.clone(), ujake_info.clone()])
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
        price: 20.to_dec().unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(ujake_info.to_coin(50)),
            LiquidateCoin {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: ujake_info.to_coin(50),
                request_coin_denom: uosmo_info.denom,
            },
        ],
        &[ujake_info.to_coin(50)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.coins.len(), 3);
    let osmo_balance = get_coin("uosmo", &position.coins);
    assert_eq!(osmo_balance.amount, Uint128::new(190));
    let atom_balance = get_coin("uatom", &position.coins);
    assert_eq!(atom_balance.amount, Uint128::new(100));
    let jake_balance = get_coin("ujake", &position.coins);
    assert_eq!(jake_balance.amount, Uint128::new(10));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(101));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.coins.len(), 2);
    assert_eq!(position.debts.len(), 0);
    let jake_balance = get_coin("ujake", &position.coins);
    assert_eq!(jake_balance.amount, Uint128::new(39));
    let osmo_balance = get_coin("uosmo", &position.coins);
    assert_eq!(osmo_balance.amount, Uint128::new(110));
}

#[test]
fn test_debt_amount_adjusted_to_max_allowed_by_request_coin() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .max_close_factor(Decimal::from_atomics(1u128, 1).unwrap())
        .allowed_coins(&[uosmo_info.clone(), uatom_info.clone()])
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
        ],
        &[Coin::new(300, uosmo_info.denom.clone())],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        denom: uatom_info.denom.clone(),
        price: 20.to_dec().unwrap(),
    });

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(50)),
            LiquidateCoin {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(50),
                request_coin_denom: uosmo_info.denom,
            },
        ],
        &[uatom_info.to_coin(50)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.coins.len(), 2);
    let osmo_balance = get_coin("uosmo", &position.coins);
    assert_eq!(osmo_balance.amount, Uint128::new(47));
    let atom_balance = get_coin("uatom", &position.coins);
    assert_eq!(atom_balance.amount, Uint128::new(100));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(98));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.coins.len(), 2);
    assert_eq!(position.debts.len(), 0);
    let atom_balance = get_coin("uatom", &position.coins);
    assert_eq!(atom_balance.amount, Uint128::new(47));
    let osmo_balance = get_coin("uosmo", &position.coins);
    assert_eq!(osmo_balance.amount, Uint128::new(253));
}

#[test]
fn test_debt_amount_no_adjustment() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .max_close_factor(Decimal::from_atomics(1u128, 1).unwrap())
        .allowed_coins(&[uosmo_info.clone(), uatom_info.clone()])
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
        ],
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
            LiquidateCoin {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(10),
                request_coin_denom: uosmo_info.denom,
            },
        ],
        &[uatom_info.to_coin(10)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.coins.len(), 2);
    let osmo_balance = get_coin("uosmo", &position.coins);
    assert_eq!(osmo_balance.amount, Uint128::new(68));
    let atom_balance = get_coin("uatom", &position.coins);
    assert_eq!(atom_balance.amount, Uint128::new(100));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(91));

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.coins.len(), 1);
    assert_eq!(position.debts.len(), 0);
    let osmo_balance = get_coin("uosmo", &position.coins);
    assert_eq!(osmo_balance.amount, Uint128::new(232));
}

// TODO: After swap is implemented, attempt to liquidate with no deposited funds:
// - Borrow atom
// - Liquidate and collect osmo
// - Swap osmo for atom
// - Repay debt
// - Withdraw
#[test]
fn test_liquidate_with_no_deposited_funds() {}
