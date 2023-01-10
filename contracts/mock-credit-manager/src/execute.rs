use cosmwasm_std::{DepsMut, Response, StdResult};
use mars_health::HealthResponse;

use crate::state::HEALTH_RESPONSES;

pub fn set_health_response(
    deps: DepsMut,
    account_id: String,
    response: HealthResponse,
) -> StdResult<Response> {
    HEALTH_RESPONSES.save(deps.storage, &account_id, &response)?;
    Ok(Response::new())
}
