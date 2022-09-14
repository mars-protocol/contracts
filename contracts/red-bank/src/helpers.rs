use std::collections::HashMap;

use cosmwasm_std::{StdResult, Uint128};

use mars_outpost::red_bank::Position;

pub fn get_uncollaterized_debt(positions: &HashMap<String, Position>) -> StdResult<Uint128> {
    positions.values().try_fold(Uint128::zero(), |total, p| {
        if p.uncollateralized_debt {
            total.checked_add(p.debt_amount)?;
        }
        Ok(total)
    })
}
