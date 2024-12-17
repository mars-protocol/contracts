use cosmwasm_std::{DepsMut, Response};
use cw2::{assert_contract_version, set_contract_version};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
};

const FROM_VERSION: &str = "2.0.1";

pub mod v1_state {
    use cosmwasm_std::{Addr, DepsMut, Uint128};
    use cw_storage_plus::Map;

    pub const UNCOLLATERALIZED_LOAN_LIMITS: Map<(&Addr, &str), Uint128> = Map::new("limits");

    /// Clear old state so we can re-use the keys
    pub fn clear_state(deps: &mut DepsMut) {
        UNCOLLATERALIZED_LOAN_LIMITS.clear(deps.storage);
    }
}

pub fn migrate(mut deps: DepsMut) -> Result<Response, ContractError> {
    // Make sure we're migrating the correct contract and from the correct version
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    // Clear old state
    v1_state::clear_state(&mut deps);

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}
