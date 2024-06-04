use cosmwasm_std::{Coin, Deps, StdResult};
use mars_types::incentives::StakedLpPositionResponse;

use crate::state::{PENDING_ASTROPORT_REWARDS, UNCLAIMED_REWARDS};

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

pub fn query_pending_astroport_rewards(
    deps: Deps,
    account_id: String,
    lp_denom: String,
) -> StdResult<Vec<Coin>> {
    Ok(PENDING_ASTROPORT_REWARDS
        .may_load(deps.storage, (account_id, lp_denom))?
        .unwrap_or_default())
}

pub fn query_staked_lp_position(
    deps: Deps,
    account_id: String,
    lp_denom: String,
) -> StdResult<StakedLpPositionResponse> {
    let staked_coin = query_staked_amount(deps, account_id.clone(), lp_denom.clone())?;
    let rewards = query_pending_astroport_rewards(deps, account_id, lp_denom)?;
    
    Ok(StakedLpPositionResponse {
        lp_coin: staked_coin,
        rewards,
    })
}

pub fn query_staked_amount(deps: Deps, account_id: String, lp_denom: String) -> StdResult<Coin> {
    let staked_amount = crate::state::STAKED_LP_POSITIONS
        .may_load(deps.storage, (account_id, lp_denom.clone()))?
        .unwrap_or_default();
    Ok(Coin {
        denom: lp_denom,
        amount: staked_amount,
    })
}
