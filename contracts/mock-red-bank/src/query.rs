use cosmwasm_std::{Deps, StdResult, Uint128};
use mars_outpost::red_bank::Market;

use crate::helpers::load_debt_amount;
use crate::state::COIN_MARKET_INFO;

use mars_outpost::red_bank::UserDebtResponse;

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

pub fn query_market(deps: Deps, denom: String) -> StdResult<Market> {
    let market_info = COIN_MARKET_INFO.load(deps.storage, denom)?;
    Ok(Market {
        max_loan_to_value: market_info.max_ltv,
        liquidation_threshold: market_info.liquidation_threshold,
        liquidation_bonus: market_info.liquidation_bonus,
        ..Default::default()
    })
}
