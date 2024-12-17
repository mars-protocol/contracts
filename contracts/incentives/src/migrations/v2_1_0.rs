use cosmwasm_std::{DepsMut, Response};
use cw2::{assert_contract_version, set_contract_version};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
};

const FROM_VERSION: &str = "2.0.0";

pub mod v1_state {
    use cosmwasm_std::{Addr, Decimal, DepsMut, Uint128};
    use cw_storage_plus::Map;

    /// Don't care about the actual types, just use some dummy types to clear the storage
    pub const ASSET_INCENTIVES: Map<&str, String> = Map::new("incentives");
    pub const USER_ASSET_INDICES: Map<(&Addr, &str), Decimal> = Map::new("indices");
    pub const USER_UNCLAIMED_REWARDS: Map<&Addr, Uint128> = Map::new("unclaimed_rewards");
    pub const USER_UNCLAIMED_REWARDS_BACKUP: Map<&Addr, Uint128> = Map::new("ur_backup");

    /// Clear old state so we can re-use the keys
    pub fn clear_state(deps: &mut DepsMut) {
        ASSET_INCENTIVES.clear(deps.storage);
        USER_ASSET_INDICES.clear(deps.storage);
        USER_UNCLAIMED_REWARDS.clear(deps.storage);
        USER_UNCLAIMED_REWARDS_BACKUP.clear(deps.storage);
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