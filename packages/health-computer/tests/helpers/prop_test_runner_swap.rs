use cosmwasm_std::{Coin, StdResult, Uint128};
use mars_rover::msg::query::DebtAmount;
use mars_rover_health_computer::HealthComputer;
use mars_rover_health_types::SwapKind;
use proptest::{
    strategy::Strategy,
    test_runner::{Config, TestRunner},
};

use super::random_health_computer;

pub fn max_swap_prop_test_runner(cases: u32, kind: &SwapKind) {
    let config = Config::with_cases(cases);

    let mut runner = TestRunner::new(config);
    runner
        .run(
            &random_health_computer().prop_filter(
                "For swap we need to ensure 2 available denom params and 1 valid deposit",
                |h| {
                    if h.denoms_data.params.len() < 2 {
                        false
                    } else {
                        let from_denom = h.denoms_data.params.keys().next().unwrap();
                        h.positions
                            .deposits
                            .iter()
                            .map(|d| &d.denom)
                            .collect::<Vec<_>>()
                            .contains(&from_denom)
                    }
                },
            ),
            |h| {
                let from_denom = h.denoms_data.params.keys().next().unwrap();
                let to_denom = h.denoms_data.params.keys().nth(1).unwrap();

                let max_swap = h.max_swap_amount_estimate(from_denom, to_denom, kind).unwrap();

                let health_before = h.compute_health().unwrap();
                if health_before.is_above_max_ltv() {
                    assert_eq!(Uint128::zero(), max_swap);
                } else {
                    let h_new = add_swap(&h, from_denom, to_denom, max_swap)?;
                    let health_after = h_new.compute_health().unwrap();

                    // Ensure still healthy
                    assert!(!health_after.is_above_max_ltv());
                }
                Ok(())
            },
        )
        .unwrap();
}

fn add_swap(
    h: &HealthComputer,
    from_denom: &str,
    to_denom: &str,
    amount: Uint128,
) -> StdResult<HealthComputer> {
    let mut new_h = h.clone();

    let from_deposit_coin_index =
        new_h.positions.deposits.iter().position(|c| c.denom == from_denom).unwrap();
    let from_lend_coin_index = new_h.positions.lends.iter().position(|c| c.denom == from_denom);

    let from_deposit_coin = new_h.positions.deposits.get_mut(from_deposit_coin_index).unwrap();
    let mut from_lend_coin = &mut Coin::default();
    if let Some(from_lend_coin_index) = from_lend_coin_index {
        from_lend_coin = new_h.positions.lends.get_mut(from_lend_coin_index).unwrap();
    }
    let from_price = new_h.denoms_data.prices.get(from_denom).unwrap();
    let to_price = new_h.denoms_data.prices.get(to_denom).unwrap();

    // Subtract the amount from current deposited and lent balance
    let total_amount = from_deposit_coin.amount + from_lend_coin.amount;
    if amount < from_deposit_coin.amount {
        from_deposit_coin.amount -= amount;
    } else if amount < total_amount {
        let remaining_from_lends = amount - from_deposit_coin.amount;
        from_deposit_coin.amount = Uint128::zero();
        from_lend_coin.amount -= remaining_from_lends;
    } else {
        // If there the amount is larger than the balance of the coin, we need to add the remaining to the debts.
        let debt_amount = amount - total_amount;
        new_h.positions.deposits.remove(from_deposit_coin_index);
        if let Some(idx) = from_lend_coin_index {
            new_h.positions.lends.remove(idx);
        }

        if let Some(debt_coin_index) =
            new_h.positions.debts.iter().position(|c| c.denom == from_denom)
        {
            let debt_coin = new_h.positions.debts.get_mut(debt_coin_index).unwrap();
            debt_coin.amount += debt_amount;
        } else {
            new_h.positions.debts.push(DebtAmount {
                denom: from_denom.to_string(),
                shares: debt_amount * Uint128::new(1000),
                amount: debt_amount,
            });
        }
    }

    // Add the swapped coins to the deposits
    let to_coin_amount = amount.mul_ceil(from_price / to_price);

    if let Some(to_coin_index) = new_h.positions.deposits.iter().position(|c| c.denom == to_denom) {
        let to_coin = new_h.positions.deposits.get_mut(to_coin_index).unwrap();
        to_coin.amount += to_coin_amount;
    } else {
        new_h.positions.deposits.push(Coin {
            denom: to_denom.to_string(),
            amount: to_coin_amount,
        });
    }

    Ok(new_h)
}
