use cosmwasm_std::{Addr, StdResult, Storage, Uint128};

use crate::state::{COLLATERAL_AMOUNT, DEBT_AMOUNT};

pub fn load_debt_amount(storage: &dyn Storage, user: &Addr, denom: &str) -> StdResult<Uint128> {
    Ok(DEBT_AMOUNT
        .may_load(storage, (user.clone(), denom.to_string()))?
        .unwrap_or_else(Uint128::zero))
}

pub fn load_collateral_amount(
    storage: &dyn Storage,
    user: &Addr,
    denom: &str,
) -> StdResult<Uint128> {
    Ok(COLLATERAL_AMOUNT
        .may_load(storage, (user.clone(), denom.to_string()))?
        .unwrap_or_else(Uint128::zero))
}
