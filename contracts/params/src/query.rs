use cosmwasm_std::{Addr, Deps, Order, StdResult};
use cw_storage_plus::Bound;

use crate::{
    state::{ASSET_PARAMS, VAULT_CONFIGS},
    types::{AssetParamsResponse, VaultConfig, VaultConfigResponse},
};

pub const DEFAULT_LIMIT: u32 = 10;

pub fn query_all_asset_params(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<AssetParamsResponse>> {
    let start = start_after.as_ref().map(|denom| Bound::exclusive(denom.as_str()));
    let limit = limit.unwrap_or(DEFAULT_LIMIT) as usize;
    ASSET_PARAMS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            let (denom, params) = res?;
            Ok(AssetParamsResponse {
                denom,
                params,
            })
        })
        .collect()
}

pub fn query_vault_config(deps: Deps, unchecked: &str) -> StdResult<VaultConfig> {
    let addr = deps.api.addr_validate(unchecked)?;
    VAULT_CONFIGS.load(deps.storage, &addr)
}

pub fn query_all_vault_configs(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<VaultConfigResponse>> {
    let vault_addr: Addr;
    let start = match &start_after {
        Some(unchecked) => {
            vault_addr = deps.api.addr_validate(unchecked)?;
            Some(Bound::exclusive(&vault_addr))
        }
        None => None,
    };

    let limit = limit.unwrap_or(DEFAULT_LIMIT) as usize;

    VAULT_CONFIGS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            let (addr, config) = res?;
            Ok(VaultConfigResponse {
                addr,
                config,
            })
        })
        .collect()
}
