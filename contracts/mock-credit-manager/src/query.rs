use cosmwasm_std::{coin, Deps, StdResult};
use mars_rover::{
    adapters::vault::VaultUnchecked,
    msg::query::{ConfigResponse, Positions, VaultInfoResponse},
};

use crate::state::{ALLOWED_COINS, CONFIG, POSITION_RESPONSES, VAULT_CONFIGS};

pub fn query_positions(deps: Deps, account_id: String) -> StdResult<Positions> {
    POSITION_RESPONSES.load(deps.storage, &account_id)
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    CONFIG.load(deps.storage)
}

pub fn query_allowed_coins(deps: Deps) -> StdResult<Vec<String>> {
    ALLOWED_COINS.load(deps.storage)
}

pub fn query_vault_info(deps: Deps, vault: VaultUnchecked) -> StdResult<VaultInfoResponse> {
    let validated = deps.api.addr_validate(&vault.address)?;
    let config = VAULT_CONFIGS.load(deps.storage, &validated)?;
    Ok(VaultInfoResponse {
        config,
        utilization: coin(1000000, "uusdc"),
        vault: VaultUnchecked::new(validated.to_string()),
    })
}
