use cosmwasm_std::{coin, Deps, Order, StdResult};
use mars_rover::{
    adapters::vault::VaultUnchecked,
    msg::query::{ConfigResponse, Positions, VaultInfoResponse},
};
use mars_rover_health_types::HealthResponse;

use crate::state::{ALLOWED_COINS, CONFIG, HEALTH_RESPONSES, POSITION_RESPONSES, VAULT_CONFIGS};

pub fn query_health(deps: Deps, account_id: String) -> StdResult<HealthResponse> {
    HEALTH_RESPONSES.load(deps.storage, &account_id)
}

pub fn query_positions(deps: Deps, account_id: String) -> StdResult<Positions> {
    POSITION_RESPONSES.load(deps.storage, &account_id)
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    CONFIG.load(deps.storage)
}

pub fn query_allowed_coins(deps: Deps) -> StdResult<Vec<String>> {
    ALLOWED_COINS.load(deps.storage)
}

pub fn query_vaults_info(deps: Deps) -> StdResult<Vec<VaultInfoResponse>> {
    VAULT_CONFIGS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|res| {
            let (addr, config) = res?;
            Ok(VaultInfoResponse {
                config,
                utilization: coin(1000000, "uusdc"),
                vault: VaultUnchecked::new(addr.to_string()),
            })
        })
        .collect()
}
