use cosmwasm_std::{Deps, StdResult};

use crate::helpers::load_debt_amount;
use crate::msg::{Market, UserAssetDebtResponse};
use crate::state::ASSET_LTV;

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
    let max_loan_to_value = ASSET_LTV.load(deps.storage, denom)?;
    Ok(Market { max_loan_to_value })
}
