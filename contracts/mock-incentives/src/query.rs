use cosmwasm_std::{Coin, Deps, StdResult};
use cw_paginate::paginate_prefix_query;
use cw_storage_plus::Bound;
use mars_types::incentives::{PaginatedStakedLpResponse, StakedLpPositionResponse};

use crate::state::{
    DEFAULT_LIMIT, MAX_LIMIT, PENDING_ASTRO_REWARDS, STAKED_ASTRO_LP_POSITIONS, UNCLAIMED_REWARDS,
};

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

pub fn query_staked_astro_lp_rewards_for_user(
    deps: Deps,
    account_id: String,
    lp_denom: String,
) -> StdResult<Vec<Coin>> {
    Ok(PENDING_ASTRO_REWARDS.may_load(deps.storage, (account_id, lp_denom))?.unwrap_or_default())
}

pub fn query_staked_lp_astro_lp_position(
    deps: Deps,
    account_id: String,
    lp_denom: String,
) -> StdResult<StakedLpPositionResponse> {
    let staked_coin = query_staked_astro_lp_amount(deps, account_id.clone(), lp_denom.clone())?;
    let rewards = query_staked_astro_lp_rewards_for_user(deps, account_id, lp_denom)?;

    Ok(StakedLpPositionResponse {
        lp_coin: staked_coin,
        rewards,
    })
}

pub fn query_all_staked_lp_positions_for_account(
    deps: Deps,
    account_id: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PaginatedStakedLpResponse> {
    let start = start_after.as_ref().map(|denom| Bound::exclusive(denom.as_str()));
    let limit: u32 = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);

    paginate_prefix_query(
        &STAKED_ASTRO_LP_POSITIONS,
        deps.storage,
        account_id.clone(),
        start,
        Some(limit),
        |denom, amount| {
            let lp_coin = Coin {
                denom,
                amount,
            };
            let rewards = query_staked_astro_lp_rewards_for_user(
                deps,
                account_id.clone(),
                lp_coin.denom.clone(),
            )?;

            Ok(StakedLpPositionResponse {
                lp_coin,
                rewards,
            })
        },
    )
}

pub fn query_staked_astro_lp_amount(
    deps: Deps,
    account_id: String,
    lp_denom: String,
) -> StdResult<Coin> {
    let staked_amount = crate::state::STAKED_ASTRO_LP_POSITIONS
        .may_load(deps.storage, (account_id, lp_denom.clone()))?
        .unwrap_or_default();
    Ok(Coin {
        denom: lp_denom,
        amount: staked_amount,
    })
}
