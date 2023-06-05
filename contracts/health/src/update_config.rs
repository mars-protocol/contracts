use cosmwasm_std::{DepsMut, MessageInfo, Response};
use mars_rover_health_types::HealthResult;

use crate::state::{CREDIT_MANAGER, OWNER, PARAMS};

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    credit_manager_opt: Option<String>,
    params_opt: Option<String>,
) -> HealthResult<Response> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let mut response = Response::new().add_attribute("action", "update_config");

    if let Some(cm) = credit_manager_opt {
        let validated = deps.api.addr_validate(&cm)?;
        CREDIT_MANAGER.save(deps.storage, &validated)?;

        response = response.add_attribute("key", "credit_manager_addr").add_attribute("value", cm);
    }

    if let Some(params) = params_opt {
        let validated = deps.api.addr_validate(&params)?;
        PARAMS.save(deps.storage, &validated)?;

        response = response.add_attribute("key", "params_addr").add_attribute("value", params);
    }

    Ok(response)
}
