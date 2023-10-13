use cosmwasm_std::{DepsMut, MessageInfo, Response};
use mars_owner::OwnerUpdate;
use mars_types::red_bank::CreateOrUpdateConfig;
use mars_utils::helpers::option_string_to_addr;

use crate::{
    error::ContractError,
    state::{CONFIG, OWNER},
};

pub fn update_owner(
    deps: DepsMut,
    info: MessageInfo,
    update: OwnerUpdate,
) -> Result<Response, ContractError> {
    Ok(OWNER.update(deps, info, update)?)
}

/// Update config
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_config: CreateOrUpdateConfig,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let mut config = CONFIG.load(deps.storage)?;

    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let CreateOrUpdateConfig {
        address_provider,
    } = new_config;

    // Update config
    config.address_provider =
        option_string_to_addr(deps.api, address_provider, config.address_provider)?;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}
