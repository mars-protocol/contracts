use cosmwasm_std::{DepsMut, Response, StdResult};
use mars_rover::msg::query::Positions;
use mars_rover_health_types::AccountKind;

use crate::state::{ACCOUNT_KINDS, POSITION_RESPONSES};

pub fn set_position_response(
    deps: DepsMut,
    account_id: String,
    positions: Positions,
) -> StdResult<Response> {
    POSITION_RESPONSES.save(deps.storage, &account_id, &positions)?;
    Ok(Response::new())
}

pub fn set_account_kind_response(
    deps: DepsMut,
    account_id: String,
    kind: AccountKind,
) -> StdResult<Response> {
    ACCOUNT_KINDS.save(deps.storage, &account_id, &kind)?;
    Ok(Response::new())
}
