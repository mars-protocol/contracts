use cosmwasm_std::{Deps, StdResult, Uint128};
use mars_red_bank_types::{
    red_bank::{Market, PaginatedUserCollateralResponse, UserCollateralResponse, UserDebtResponse},
    Metadata,
};

use crate::{
    helpers::{load_collateral_amount, load_collateral_denoms, load_debt_amount},
    state::MARKETS,
};

pub fn query_market(deps: Deps, denom: String) -> StdResult<Market> {
    MARKETS.load(deps.storage, &denom)
}

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
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<UserCollateralResponse>> {
    let res = query_collaterals_v2(deps, user, account_id, start_after, limit)?;
    Ok(res.data)
}

pub fn query_collaterals_v2(
    deps: Deps,
    user: String,
    account_id: Option<String>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PaginatedUserCollateralResponse> {
    let denoms =
        load_collateral_denoms(deps.storage, &user, &account_id.clone().unwrap_or_default())?;
    let limit = limit.unwrap_or(5) as usize; // red-bank can have different value as default, we only use it to validate if pagination works as expected

    let (start_index, has_more) = match start_after {
        Some(sa) => {
            let start_index = denoms.iter().position(|denom| denom == &sa).unwrap_or(denoms.len());
            let has_more = start_index + 1 < denoms.len();
            (start_index + 1, has_more)
        }
        None => (0, denoms.len() > limit),
    };

    let collaterals = denoms
        .iter()
        .skip(start_index)
        .take(limit)
        .map(|denom| {
            let amount = load_collateral_amount(
                deps.storage,
                &user,
                &account_id.clone().unwrap_or_default(),
                denom,
            )?;
            Ok(UserCollateralResponse {
                denom: denom.clone(),
                amount_scaled: Default::default(),
                amount,
                enabled: true,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(PaginatedUserCollateralResponse {
        data: collaterals,
        metadata: Metadata {
            has_more,
        },
    })
}
