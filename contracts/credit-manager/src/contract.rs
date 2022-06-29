use std::convert::TryFrom;

use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw_asset::{AssetInfo, AssetInfoKey, AssetInfoUnchecked};

use cw_storage_plus::Bound;
use rover::{ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::state::{ALLOWED_ASSETS, ALLOWED_VAULTS, OWNER};

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let owner = deps.api.addr_validate(&msg.owner)?;
    OWNER.save(deps.storage, &owner)?;

    msg
        .allowed_vaults
        .iter()
        .try_for_each(|vault| {
            ALLOWED_VAULTS.save(deps.storage, deps.api.addr_validate(vault)?, &true)
        })?;

    msg
        .allowed_assets
        .iter()
        .try_for_each(|info| {
            ALLOWED_ASSETS.save(deps.storage, info.check(deps.api, None)?.into(), &true)
        })?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(_: DepsMut, _env: Env, _: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {}
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Owner {} => to_binary(&query_owner(deps)?),
        QueryMsg::AllowedVaults {
            start_after,
            limit,
        } => to_binary(&query_allowed_vaults(deps, start_after, limit)?),
        QueryMsg::AllowedAssets {
            start_after,
            limit,
        } => to_binary(&query_allowed_assets(deps, start_after, limit)?),
    }
}

fn query_owner(deps: Deps) -> StdResult<String> {
    Ok(OWNER.load(deps.storage)?.into())
}

/// NOTE: This implementation of the query function assumes the map `ALLOWED_VAULTS` only saves `true`.
/// If a vault is to be removed from the whitelist, the map must remove the correspoinding key, instead
/// of setting the value to `false`.
fn query_allowed_vaults(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<String>> {
    let addr: Addr;
    let start = match &start_after {
        Some(addr_str) => {
            addr = deps.api.addr_validate(addr_str)?;
            Some(Bound::exclusive(addr))
        },
        None => None,
    };

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    ALLOWED_VAULTS
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|vault_addr| vault_addr.to_string()))
        .collect()
}

/// NOTE: This implementation of the query function assumes the map `ALLOWED_ASSETS` only saves `true`.
/// If an asset is to be removed from the whitelist, the map must remove the correspoinding key, instead
/// of setting the value to `false`.
fn query_allowed_assets(
    deps: Deps,
    start_after: Option<AssetInfoUnchecked>,
    limit: Option<u32>
) -> StdResult<Vec<AssetInfoUnchecked>> {
    let info: AssetInfo;
    let start = match &start_after {
        Some(unchecked) => {
            info = unchecked.check(deps.api, None)?;
            Some(Bound::exclusive(AssetInfoKey::from(info)))
        },
        None => None,
    };

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    ALLOWED_ASSETS
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?
        .into_iter()
        .map(|key| AssetInfoUnchecked::try_from(key))
        .collect()
}
