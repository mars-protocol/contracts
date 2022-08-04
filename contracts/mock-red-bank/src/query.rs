use cosmwasm_std::{Deps, StdResult};

use crate::helpers::load_debt_amount;
use crate::msg::UserAssetDebtResponse;

pub fn query_debt(
    deps: Deps,
    user_address: String,
    denom: String,
) -> StdResult<UserAssetDebtResponse> {
    let user_addr = deps.api.addr_validate(&user_address)?;
    let amount = load_debt_amount(deps.storage, &user_addr, &denom)?;
    Ok(UserAssetDebtResponse { denom, amount })
}
