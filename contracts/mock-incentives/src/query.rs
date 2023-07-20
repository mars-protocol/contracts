use cosmwasm_std::{Coin, Deps, StdResult};

use crate::state::UNCLAIMED_REWARDS;

pub fn query_unclaimed_rewards(
    deps: Deps,
    user: &str,
    account_id: &Option<String>,
) -> StdResult<Vec<Coin>> {
    let user_addr = deps.api.addr_validate(user)?;
    Ok(UNCLAIMED_REWARDS
        .may_load(deps.storage, (user_addr, account_id.clone().unwrap_or_default()))?
        .unwrap_or_default())
}
