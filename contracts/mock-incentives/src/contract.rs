#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use mars_red_bank_types::incentives;

use crate::{
    execute::{balance_change, claim_rewards},
    query::query_unclaimed_rewards,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: incentives::ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        incentives::ExecuteMsg::ClaimRewards {
            account_id,
            ..
        } => claim_rewards(deps, info, account_id),
        incentives::ExecuteMsg::BalanceChange {
            user_addr,
            account_id,
            denom,
            user_amount_scaled_before,
            ..
        } => balance_change(deps, info, user_addr, account_id, denom, user_amount_scaled_before),
        _ => unimplemented!("Msg not supported!"),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: incentives::QueryMsg) -> StdResult<Binary> {
    match msg {
        incentives::QueryMsg::UserUnclaimedRewards {
            user,
            account_id,
            ..
        } => to_binary(&query_unclaimed_rewards(deps, &user, &account_id)?),
        _ => unimplemented!("Query not supported!"),
    }
}
