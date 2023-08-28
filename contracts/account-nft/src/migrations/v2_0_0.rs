use cosmwasm_std::{DepsMut, Empty, Response};
use cw2::set_contract_version;

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    nft_config::NftConfig,
    state::CONFIG,
};

const FROM_VERSION: &str = "1.0.0";

pub mod v1_state {
    use cw_storage_plus::Item;
    use mars_rover_old::adapters::account_nft::NftConfig;

    pub const CONFIG: Item<NftConfig> = Item::new("config");
}

pub fn migrate(deps: DepsMut) -> Result<Response, ContractError> {
    // make sure we're migrating the correct contract and from the correct version
    cw2::assert_contract_version(
        deps.as_ref().storage,
        &format!("crates.io:{CONTRACT_NAME}"),
        FROM_VERSION,
    )?;

    // CONFIG updated, re-initializing
    let old_config_state = v1_state::CONFIG.load(deps.storage)?;
    v1_state::CONFIG.remove(deps.storage);
    CONFIG.save(
        deps.storage,
        &NftConfig {
            max_value_for_burn: old_config_state.max_value_for_burn,
            health_contract_addr: None, // this can be updated via update_config
        },
    )?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    Ok(cw721_base::upgrades::v0_17::migrate::<Empty, Empty, Empty, Empty>(deps)?)
}
