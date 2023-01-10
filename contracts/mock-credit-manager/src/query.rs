use crate::state::HEALTH_RESPONSES;
use cosmwasm_std::{Deps, StdResult};
use mars_health::HealthResponse;

pub fn query_health(deps: Deps, account_id: String) -> StdResult<HealthResponse> {
    HEALTH_RESPONSES.load(deps.storage, &account_id)
}
