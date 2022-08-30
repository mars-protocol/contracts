use std::collections::HashMap;

use cosmwasm_std::{Deps, StdError, StdResult, Uint128};

use mars_outpost::red_bank::{Market, Position};

use crate::state::{MARKETS, MARKET_DENOMS_BY_INDEX};

pub fn get_market_from_index(deps: &Deps, index: u32) -> StdResult<(String, Market)> {
    let denom = MARKET_DENOMS_BY_INDEX
        .load(deps.storage, index)
        .map_err(|_| StdError::generic_err(format!("no denom exists with index: {}", index)))?;

    let market = MARKETS
        .load(deps.storage, &denom)
        .map_err(|_| StdError::generic_err(format!("no market exists with denom: {}", denom)))?;

    Ok((denom, market))
}

// bitwise operations
/// Gets bit: true: 1, false: 0
pub fn get_bit(bitmap: Uint128, index: u32) -> StdResult<bool> {
    if index >= 128 {
        return Err(StdError::generic_err("index out of range"));
    }
    Ok(((bitmap.u128() >> index) & 1) == 1)
}

/// Sets bit to 1
pub fn set_bit(bitmap: &mut Uint128, index: u32) -> StdResult<()> {
    if index >= 128 {
        return Err(StdError::generic_err("index out of range"));
    }
    *bitmap = Uint128::from(bitmap.u128() | (1 << index));
    Ok(())
}

/// Sets bit to 0
pub fn unset_bit(bitmap: &mut Uint128, index: u32) -> StdResult<()> {
    if index >= 128 {
        return Err(StdError::generic_err("index out of range"));
    }
    *bitmap = Uint128::from(bitmap.u128() & !(1 << index));
    Ok(())
}

pub fn get_uncollaterized_debt(positions: &HashMap<String, Position>) -> StdResult<Uint128> {
    positions.values().try_fold(Uint128::zero(), |total, p| {
        if p.uncollateralized_debt {
            total.checked_add(p.debt_amount)?;
        }
        Ok(total)
    })
}
