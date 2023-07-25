use cosmwasm_std::{Coin, StdResult, Uint128};
use mars_rover::msg::query::DebtAmount;
use mars_rover_health_computer::HealthComputer;
use mars_rover_health_types::BorrowTarget;
use proptest::test_runner::{Config, TestRunner};

use super::random_health_computer;

pub fn max_borrow_prop_test_runner(cases: u32, target: &BorrowTarget) {
    let config = Config::with_cases(cases);

    let mut runner = TestRunner::new(config);
    runner
        .run(&random_health_computer(), |h| {
            let denom_to_borrow = h.denoms_data.params.keys().next().unwrap();
            let max_borrow = h.max_borrow_amount_estimate(denom_to_borrow, target).unwrap();

            let health_before = h.compute_health().unwrap();
            if health_before.is_above_max_ltv() {
                assert_eq!(Uint128::zero(), max_borrow);
            } else {
                let h_new = add_borrow(&h, denom_to_borrow, max_borrow)?;
                let health_after = h_new.compute_health().unwrap();

                // Ensure still healthy
                assert!(!health_after.is_above_max_ltv());
            }
            Ok(())
        })
        .unwrap();
}

fn add_borrow(h: &HealthComputer, denom: &str, amount: Uint128) -> StdResult<HealthComputer> {
    let mut new_h = h.clone();
    new_h.positions.debts.push(DebtAmount {
        denom: denom.to_string(),
        shares: amount * Uint128::new(1000),
        amount,
    });
    new_h.positions.deposits.push(Coin {
        denom: denom.to_string(),
        amount,
    });
    Ok(new_h)
}
