use std::convert::TryInto;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};

use cw_storage_plus::Bound;
use mars_outpost::address_provider::{
    Config, ExecuteMsg, InstantiateMsg, LocalAddressResponse, MarsLocal, MarsRemote, QueryMsg,
    RemoteAddressResponse,
};

use crate::error::ContractError;
use crate::helpers::{assert_owner, assert_valid_addr};
use crate::key::MarsAddressKey;
use crate::state::{CONFIG, LOCAL_ADDRESSES, REMOTE_ADDRESSES};

pub const CONTRACT_NAME: &str = "crates.io:mars-address-provider";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

// INIT

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    assert_valid_addr(deps.api, &msg.owner, &msg.prefix)?;

    CONFIG.save(deps.storage, &msg)?;

    Ok(Response::default())
}

// EXECUTE

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SetLocalAddress {
            local,
            address,
        } => set_local_address(deps, info.sender, local, address),
        ExecuteMsg::SetRemoteAddress {
            remote,
            address,
        } => set_remote_address(deps, info.sender, remote, address),
        ExecuteMsg::TransferOwnership {
            new_owner,
        } => transfer_ownership(deps, info.sender, new_owner),
    }
}

pub fn set_local_address(
    deps: DepsMut,
    sender: Addr,
    local: MarsLocal,
    address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    assert_owner(&sender, &config.owner)?;
    let validated_address = deps.api.addr_validate(&address)?;

    LOCAL_ADDRESSES.save(deps.storage, local.into(), &validated_address)?;

    Ok(Response::new()
        .add_attribute("action", "outposts/address-provider/set_local_address")
        .add_attribute("local", local.to_string())
        .add_attribute("address", address))
}

pub fn set_remote_address(
    deps: DepsMut,
    sender: Addr,
    remote: MarsRemote,
    address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    assert_owner(&sender, &config.owner)?;
    assert_valid_addr(deps.api, &address, &config.prefix)?;

    REMOTE_ADDRESSES.save(deps.storage, remote.into(), &address)?;

    Ok(Response::new()
        .add_attribute("action", "outposts/address-provider/set_remote_address")
        .add_attribute("remote", remote.to_string())
        .add_attribute("address", address))
}

pub fn transfer_ownership(
    deps: DepsMut,
    sender: Addr,
    new_owner: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    assert_owner(&sender, &config.owner)?;
    assert_valid_addr(deps.api, &new_owner, &config.prefix)?;

    config.owner = new_owner.clone();
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "outposts/address-provider/transfer_ownership")
        .add_attribute("previous_owner", sender)
        .add_attribute("new_owner", new_owner))
}

// QUERIES

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::LocalAddress(local) => to_binary(&query_local_address(deps, local)?),
        QueryMsg::LocalAddresses(locals) => to_binary(&query_local_addresses(deps, locals)?),
        QueryMsg::AllLocalAddresses {
            start_after,
            limit,
        } => to_binary(&query_all_local_addresses(deps, start_after, limit)?),
        QueryMsg::RemoteAddress(remote) => to_binary(&query_remote_address(deps, remote)?),
        QueryMsg::RemoteAddresses(remotes) => to_binary(&query_remote_addresses(deps, remotes)?),
        QueryMsg::AllRemoteAddresses {
            start_after,
            limit,
        } => to_binary(&query_all_remote_addresses(deps, start_after, limit)?),
    }
}

fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

fn query_local_address(deps: Deps, local: MarsLocal) -> StdResult<LocalAddressResponse> {
    Ok(LocalAddressResponse {
        local,
        address: LOCAL_ADDRESSES.load(deps.storage, local.into())?,
    })
}

fn query_local_addresses(
    deps: Deps,
    locals: Vec<MarsLocal>,
) -> StdResult<Vec<LocalAddressResponse>> {
    locals.into_iter().map(|local| query_local_address(deps, local)).collect::<StdResult<Vec<_>>>()
}

fn query_all_local_addresses(
    deps: Deps,
    start_after: Option<MarsLocal>,
    limit: Option<u32>,
) -> StdResult<Vec<LocalAddressResponse>> {
    let start = start_after.map(MarsAddressKey::from).map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    LOCAL_ADDRESSES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            Ok(LocalAddressResponse {
                local: k.try_into()?,
                address: v,
            })
        })
        .collect()
}

fn query_remote_address(deps: Deps, remote: MarsRemote) -> StdResult<RemoteAddressResponse> {
    Ok(RemoteAddressResponse {
        remote,
        address: REMOTE_ADDRESSES.load(deps.storage, remote.into())?,
    })
}

fn query_remote_addresses(
    deps: Deps,
    remotes: Vec<MarsRemote>,
) -> StdResult<Vec<RemoteAddressResponse>> {
    remotes.into_iter().map(|gov| query_remote_address(deps, gov)).collect::<StdResult<Vec<_>>>()
}

fn query_all_remote_addresses(
    deps: Deps,
    start_after: Option<MarsRemote>,
    limit: Option<u32>,
) -> StdResult<Vec<RemoteAddressResponse>> {
    let start = start_after.map(MarsAddressKey::from).map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    REMOTE_ADDRESSES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            Ok(RemoteAddressResponse {
                remote: k.try_into()?,
                address: v,
            })
        })
        .collect()
}
