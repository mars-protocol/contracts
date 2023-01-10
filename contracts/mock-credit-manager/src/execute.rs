use crate::state::HEALTH_RESPONSES;
use cosmwasm_std::{DepsMut, Response, StdResult};
use mars_health::HealthResponse;

pub fn set_health_response(
    deps: DepsMut,
    account_id: String,
    response: HealthResponse,
) -> StdResult<Response> {
    HEALTH_RESPONSES.save(deps.storage, &account_id, &response)?;
    Ok(Response::new())
}
