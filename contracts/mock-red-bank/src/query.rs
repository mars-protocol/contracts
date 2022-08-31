use cosmwasm_std::{Deps, StdResult};
use mars_outpost::red_bank::Market;

use crate::helpers::load_debt_amount;
use crate::msg::UserAssetDebtResponse;
use crate::state::COIN_MARKET_INFO;

pub fn query_debt(
    deps: Deps,
    user_address: String,
    denom: String,
) -> StdResult<UserAssetDebtResponse> {
    let user_addr = deps.api.addr_validate(&user_address)?;
    let amount = load_debt_amount(deps.storage, &user_addr, &denom)?;
    Ok(UserAssetDebtResponse { denom, amount })
}

pub fn query_market(deps: Deps, denom: String) -> StdResult<Market> {
    let market_info = COIN_MARKET_INFO.load(deps.storage, denom)?;
    Ok(Market {
        max_loan_to_value: market_info.max_ltv,
        liquidation_threshold: market_info.liquidation_threshold,
        ..Default::default()
    })
}
