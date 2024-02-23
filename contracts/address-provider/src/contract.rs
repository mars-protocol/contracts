use std::convert::TryInto;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Order, Response,
    StdResult,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;
use mars_owner::{OwnerInit::SetInitialOwner, OwnerUpdate};
use mars_types::address_provider::{
    AddressResponseItem, Config, ConfigResponse, ExecuteMsg, InstantiateMsg, MarsAddressType,
    QueryMsg,
};

use crate::{
    error::ContractError,
    helpers::{assert_valid_addr, assert_valid_prefix},
    key::MarsAddressTypeKey,
    migrations,
    state::{ADDRESSES, CONFIG, OWNER},
};

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
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
    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    assert_valid_prefix(&msg.owner, &msg.prefix)?;

    OWNER.initialize(
        deps.storage,
        deps.api,
        SetInitialOwner {
            owner: msg.owner,
        },
    )?;

    CONFIG.save(
        deps.storage,
        &Config {
            prefix: msg.prefix,
        },
    )?;

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
        ExecuteMsg::UpdateOwner(update) => update_owner(deps, info, update),
    }
}

fn set_address(
    deps: DepsMut,
    sender: Addr,
    address_type: MarsAddressType,
    address: String,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &sender)?;

    let config = CONFIG.load(deps.storage)?;
    assert_valid_addr(deps.api, &address, &config.prefix)?;

    ADDRESSES.save(deps.storage, address_type.into(), &address)?;

    Ok(Response::new()
        .add_attribute("action", "set_address")
        .add_attribute("address_type", address_type.to_string())
        .add_attribute("address", address))
}

fn update_owner(
    deps: DepsMut,
    info: MessageInfo,
    update: OwnerUpdate,
) -> Result<Response, ContractError> {
    Ok(OWNER.update(deps, info, update)?)
}

// QUERIES

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Address(address_type) => to_json_binary(&query_address(deps, address_type)?),
        QueryMsg::Addresses(address_types) => {
            to_json_binary(&query_addresses(deps, address_types)?)
        }
        QueryMsg::AllAddresses {
            start_after,
            limit,
        } => to_json_binary(&query_all_addresses(deps, start_after, limit)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let owner_state = OWNER.query(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: owner_state.owner,
        proposed_new_owner: owner_state.proposed,
        prefix: config.prefix,
    })
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
    migrations::v2_0_0::migrate(deps)
}
