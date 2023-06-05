use cosmwasm_std::{Deps, StdResult, Uint128};
use mars_red_bank_types::red_bank::{UserCollateralResponse, UserDebtResponse};

use crate::helpers::{load_collateral_amount, load_debt_amount};

pub fn query_debt(deps: Deps, user: String, denom: String) -> StdResult<UserDebtResponse> {
    let user_addr = deps.api.addr_validate(&user)?;
    let amount = load_debt_amount(deps.storage, &user_addr, &denom)?;
    Ok(UserDebtResponse {
        denom,
        amount,
        amount_scaled: Uint128::zero(),
        uncollateralized: false,
    })
}

pub fn query_collateral(
    deps: Deps,
    user: String,
    denom: String,
) -> StdResult<UserCollateralResponse> {
    let user_addr = deps.api.addr_validate(&user)?;
    let amount = load_collateral_amount(deps.storage, &user_addr, &denom)?;
    Ok(UserCollateralResponse {
        denom,
        amount,
        amount_scaled: Default::default(),
        enabled: true,
    })
}
