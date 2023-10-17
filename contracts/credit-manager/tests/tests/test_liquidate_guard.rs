use cosmwasm_std::{coins, Addr};
use mars_credit_manager::error::ContractError;
use mars_types::credit_manager::{
    Action::{Borrow, Deposit, Liquidate},
    LiquidateRequest,
};

use super::helpers::{assert_err, uatom_info, uosmo_info, AccountToFund, MockEnv};

#[test]
fn cannot_liquidate_own_account() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let liquidator = Addr::unchecked("liquidator");
    let mut mock = MockEnv::new()
        .set_params(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: coins(3000, uatom_info.denom.clone()),
        })
        .build()
        .unwrap();

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(100)),
            Borrow(uatom_info.to_coin(1000)),
            Liquidate {
                liquidatee_account_id: liquidator_account_id.clone(), // Should not be allowed
                debt_coin: uatom_info.to_coin(100),
                request: LiquidateRequest::Deposit(uosmo_info.denom),
            },
        ],
        &[uatom_info.to_coin(100)],
    );

    assert_err(res, ContractError::SelfLiquidation);
}
