use astroport_v5::incentives::{ExecuteMsg, QueryMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};

use crate::{
    execute::{claim_rewards, deposit, incentivise, withdraw},
    query::{query_deposit, query_rewards},
};
#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Deposit {
            recipient: _,
        } => deposit(deps, env, info),
        ExecuteMsg::Withdraw {
            lp_token,
            amount,
        } => withdraw(deps, env, info, lp_token, amount),
        ExecuteMsg::ClaimRewards {
            lp_tokens,
        } => claim_rewards(deps, env, info, lp_tokens),
        ExecuteMsg::Incentivize {
            lp_token,
            schedule,
        } => incentivise(deps, env, lp_token, schedule),
        _ => unimplemented!("Msg not supported! : {:?}", msg),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::PendingRewards {
            lp_token,
            user,
        } => to_json_binary(&query_rewards(deps, env, user, lp_token)?),
        QueryMsg::Deposit {
            lp_token,
            user,
        } => to_json_binary(&query_deposit(deps, user, lp_token)?),
        _ => panic!("Unsupported query!"),
    }
}
