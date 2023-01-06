use std::convert::TryInto;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw_storage_plus::Bound;
use mars_outpost::address_provider::{
    AddressResponseItem, Config, ExecuteMsg, InstantiateMsg, MarsAddressType, QueryMsg,
};

use crate::{
    error::ContractError,
    helpers::{assert_owner, assert_valid_addr},
    key::MarsAddressTypeKey,
    state::{ADDRESSES, CONFIG},
};

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
) -> Result<Response, ContractError> {
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
        ExecuteMsg::SetAddress {
            address_type: contract,
            address,
        } => set_address(deps, info.sender, contract, address),
        ExecuteMsg::TransferOwnership {
            new_owner,
        } => transfer_ownership(deps, info.sender, new_owner),
    }
}

pub fn set_address(
    deps: DepsMut,
    sender: Addr,
    address_type: MarsAddressType,
    address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    assert_owner(&sender, &config.owner)?;
    assert_valid_addr(deps.api, &address, &config.prefix)?;

    ADDRESSES.save(deps.storage, address_type.into(), &address)?;

    Ok(Response::new()
        .add_attribute("action", "outposts/address-provider/set_address")
        .add_attribute("address_type", address_type.to_string())
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
        QueryMsg::Address(address_type) => to_binary(&query_address(deps, address_type)?),
        QueryMsg::Addresses(address_types) => to_binary(&query_addresses(deps, address_types)?),
        QueryMsg::AllAddresses {
            start_after,
            limit,
        } => to_binary(&query_all_addresses(deps, start_after, limit)?),
    }
}

fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

fn query_address(deps: Deps, address_type: MarsAddressType) -> StdResult<AddressResponseItem> {
    Ok(AddressResponseItem {
        address_type,
        address: ADDRESSES.load(deps.storage, address_type.into())?,
    })
}

fn query_addresses(
    deps: Deps,
    address_types: Vec<MarsAddressType>,
) -> StdResult<Vec<AddressResponseItem>> {
    address_types
        .into_iter()
        .map(|address_type| query_address(deps, address_type))
        .collect::<StdResult<Vec<_>>>()
}

fn query_all_addresses(
    deps: Deps,
    start_after: Option<MarsAddressType>,
    limit: Option<u32>,
) -> StdResult<Vec<AddressResponseItem>> {
    let start = start_after.map(MarsAddressTypeKey::from).map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    ADDRESSES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            Ok(AddressResponseItem {
                address_type: k.try_into()?,
                address: v,
            })
        })
        .collect()
}
