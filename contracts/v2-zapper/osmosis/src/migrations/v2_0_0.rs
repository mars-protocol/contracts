use cosmwasm_std::{DepsMut, Response};
use cw2::set_contract_version;
use mars_zapper_base::ContractError;

use crate::contract::{CONTRACT_NAME, CONTRACT_VERSION};

const FROM_VERSION: &str = "1.0.0";

pub fn migrate(deps: DepsMut) -> Result<Response, ContractError> {
    // make sure we're migrating the correct contract and from the correct version
    cw2::assert_contract_version(
        deps.as_ref().storage,
        &format!("crates.io:{CONTRACT_NAME}"),
        FROM_VERSION,
    )?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}
