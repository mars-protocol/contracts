use cosmwasm_std::{coins, Addr, Coin, Decimal};
use mars_mock_oracle::msg::CoinPrice;
use mars_red_bank_types::oracle::ActionKind;
use mars_rover::{
    error::ContractError,
    msg::execute::{
        Action::{Borrow, Deposit, Liquidate},
        LiquidateRequest,
    },
};

use crate::helpers::{assert_err, uatom_info, uosmo_info, AccountToFund, MockEnv};

pub mod helpers;

#[test]
fn liquidation_uses_correct_price_kind() {
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

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    // The liquidation should not acknowledge DEFAULT pricing changes
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(59u128, 1).unwrap(),
    });

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(100)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(100),
                request: LiquidateRequest::Deposit(uosmo_info.denom.clone()),
            },
        ],
        &[uatom_info.to_coin(100)],
    );

    assert_err(
        res,
        ContractError::NotLiquidatable {
            account_id: liquidatee_account_id.clone(),
            lqdt_health_factor: "1.483516483516483516".to_string(),
        },
    );

    // The liquidation should acknowledge LIQUIDATION pricing changes and go through fine
    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(59u128, 1).unwrap(),
    });

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
}
