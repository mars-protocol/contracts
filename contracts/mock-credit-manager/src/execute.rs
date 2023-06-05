use cosmwasm_std::{DepsMut, Response, StdResult};
use mars_rover::msg::query::Positions;

use crate::state::POSITION_RESPONSES;

pub fn set_position_response(
    deps: DepsMut,
    account_id: String,
    positions: Positions,
) -> StdResult<Response> {
    POSITION_RESPONSES.save(deps.storage, &account_id, &positions)?;
    Ok(Response::new())
}
