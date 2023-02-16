use cosmwasm_std::{DepsMut, MessageInfo, Response};
use mars_rover_health_types::HealthResult;

use crate::state::{CREDIT_MANAGER, OWNER};

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    credit_manager: String,
) -> HealthResult<Response> {
    OWNER.assert_owner(deps.storage, &info.sender)?;
    let validated = deps.api.addr_validate(&credit_manager)?;
    CREDIT_MANAGER.save(deps.storage, &validated)?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("key", "credit_manager_addr")
        .add_attribute("value", credit_manager))
}
