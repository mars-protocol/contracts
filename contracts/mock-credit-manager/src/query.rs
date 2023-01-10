use cosmwasm_std::{Deps, StdResult};
use mars_health::HealthResponse;

use crate::state::HEALTH_RESPONSES;

pub fn query_health(deps: Deps, account_id: String) -> StdResult<HealthResponse> {
    HEALTH_RESPONSES.load(deps.storage, &account_id)
}
