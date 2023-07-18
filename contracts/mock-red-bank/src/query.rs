use cosmwasm_std::{Deps, StdResult, Uint128};
use mars_red_bank_types::red_bank::{UserCollateralResponse, UserDebtResponse};

use crate::helpers::{load_collateral_amount, load_collateral_denoms, load_debt_amount};

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
    account_id: Option<String>,
    denom: String,
) -> StdResult<UserCollateralResponse> {
    let amount =
        load_collateral_amount(deps.storage, &user, &account_id.unwrap_or_default(), &denom)?;
    Ok(UserCollateralResponse {
        denom,
        amount,
        amount_scaled: Default::default(),
        enabled: true,
    })
}

pub fn query_collaterals(
    deps: Deps,
    user: String,
    account_id: Option<String>,
) -> StdResult<Vec<UserCollateralResponse>> {
    load_collateral_denoms(deps.storage, &user, &account_id.clone().unwrap_or_default())?
        .into_iter()
        .map(|denom| {
            load_collateral_amount(
                deps.storage,
                &user,
                &account_id.clone().unwrap_or_default(),
                &denom,
            )
            .map(|amount| UserCollateralResponse {
                denom,
                amount_scaled: Default::default(),
                amount,
                enabled: true,
            })
        })
        .collect()
}
