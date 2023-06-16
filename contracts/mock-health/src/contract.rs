#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use mars_rover_health_types::{AccountKind, HealthResponse, HealthResult, QueryMsg};

use crate::{msg::ExecuteMsg, state::HEALTH_RESPONSES};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(_: DepsMut, _: Env, _: MessageInfo, _: Empty) -> HealthResult<Response> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, _: Env, _: MessageInfo, msg: ExecuteMsg) -> HealthResult<Response> {
    match msg {
        ExecuteMsg::SetHealthResponse {
            account_id,
            response,
            kind,
        } => set_health_response(deps, account_id, kind, response),
    }
}

pub fn set_health_response(
    deps: DepsMut,
    account_id: String,
    kind: AccountKind,
    response: HealthResponse,
) -> HealthResult<Response> {
    HEALTH_RESPONSES.save(deps.storage, (&account_id, &kind.to_string()), &response)?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _: Env, msg: QueryMsg) -> HealthResult<Binary> {
    let res = match msg {
        QueryMsg::Health {
            account_id,
            kind,
        } => to_binary(&query_health(deps, &account_id, kind)?),
        _ => unimplemented!("query msg not supported"),
    };
    res.map_err(Into::into)
}

pub fn query_health(deps: Deps, account_id: &str, kind: AccountKind) -> StdResult<HealthResponse> {
    HEALTH_RESPONSES.load(deps.storage, (account_id, &kind.to_string()))
}
