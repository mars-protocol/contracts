use cosmwasm_std::{DepsMut, Response};
use cw2::set_contract_version;
use mars_owner::OwnerInit::SetInitialOwner;
use mars_types::{
    error::MarsError,
    red_bank::{Config, CreateOrUpdateConfig, InstantiateMsg},
};
use mars_utils::helpers::{option_string_to_addr, zero_address};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    state::{CONFIG, OWNER},
};

pub fn instantiate(deps: DepsMut, msg: InstantiateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let CreateOrUpdateConfig {
        address_provider,
    } = msg.config;

    if address_provider.is_none() {
        return Err(MarsError::InstantiateParamsUnavailable {}.into());
    };

    let config = Config {
        address_provider: option_string_to_addr(deps.api, address_provider, zero_address())?,
    };

    CONFIG.save(deps.storage, &config)?;

    OWNER.initialize(
        deps.storage,
        deps.api,
        SetInitialOwner {
            owner: msg.owner,
        },
    )?;

    Ok(Response::default())
}
