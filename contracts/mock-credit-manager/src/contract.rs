#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use mars_rover::msg::QueryMsg;

use crate::{
    execute::{set_account_kind_response, set_position_response},
    msg::{ExecuteMsg, InstantiateMsg},
    query::{query_account_kind, query_config, query_positions},
    state::CONFIG,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    CONFIG.save(deps.storage, &msg.config)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::SetPositionsResponse {
            account_id,
            positions,
        } => set_position_response(deps, account_id, positions),
        ExecuteMsg::SetAccountKindResponse {
            account_id,
            kind,
        } => set_account_kind_response(deps, account_id, kind),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Positions {
            account_id,
        } => to_binary(&query_positions(deps, account_id)?),
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::AccountKind {
            account_id,
        } => to_binary(&query_account_kind(deps, account_id)?),
        _ => unimplemented!("query msg not supported"),
    }
}
