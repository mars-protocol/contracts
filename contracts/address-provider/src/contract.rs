use std::convert::TryInto;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};

use cw_storage_plus::Bound;
use mars_outpost::address_provider::{
    Config, ContractAddressResponse, ExecuteMsg, GovAddressResponse, InstantiateMsg, MarsContract,
    MarsGov, QueryMsg,
};

use crate::error::ContractError;
use crate::helpers::{assert_owner, assert_valid_addr};
use crate::key::MarsAddressKey;
use crate::state::{CONFIG, CONTRACTS, GOVERNANCE};

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
        ExecuteMsg::SetContractAddress {
            contract,
            address,
        } => set_contract_address(deps, info.sender, contract, address),
        ExecuteMsg::SetGovAddress {
            gov,
            address,
        } => set_gov_address(deps, info.sender, gov, address),
        ExecuteMsg::TransferOwnership {
            new_owner,
        } => transfer_ownership(deps, info.sender, new_owner),
    }
}

pub fn set_contract_address(
    deps: DepsMut,
    sender: Addr,
    contract: MarsContract,
    address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    assert_owner(&sender, &config.owner)?;
    let validated_address = deps.api.addr_validate(&address)?;

    CONTRACTS.save(deps.storage, contract.into(), &validated_address)?;

    Ok(Response::new()
        .add_attribute("action", "outposts/address-provider/set_contract_address")
        .add_attribute("contract", contract.to_string())
        .add_attribute("address", address))
}

pub fn set_gov_address(
    deps: DepsMut,
    sender: Addr,
    gov: MarsGov,
    address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    assert_owner(&sender, &config.owner)?;
    assert_valid_addr(deps.api, &address, &config.prefix)?;

    GOVERNANCE.save(deps.storage, gov.into(), &address)?;

    Ok(Response::new()
        .add_attribute("action", "outposts/address-provider/set_gov_address")
        .add_attribute("gov", gov.to_string())
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
        QueryMsg::ContractAddress(contract) => to_binary(&query_contract_address(deps, contract)?),
        QueryMsg::ContractAddresses(contracts) => {
            to_binary(&query_contract_addresses(deps, contracts)?)
        }
        QueryMsg::AllContractAddresses {
            start_after,
            limit,
        } => to_binary(&query_all_contract_addresses(deps, start_after, limit)?),
        QueryMsg::GovAddress(gov) => to_binary(&query_gov_address(deps, gov)?),
        QueryMsg::GovAddresses(gov) => to_binary(&query_gov_addresses(deps, gov)?),
        QueryMsg::AllGovAddresses {
            start_after,
            limit,
        } => to_binary(&query_all_gov_addresses(deps, start_after, limit)?),
    }
}

fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

fn query_contract_address(
    deps: Deps,
    contract: MarsContract,
) -> StdResult<ContractAddressResponse> {
    Ok(ContractAddressResponse {
        contract,
        address: CONTRACTS.load(deps.storage, contract.into())?,
    })
}

fn query_contract_addresses(
    deps: Deps,
    contracts: Vec<MarsContract>,
) -> StdResult<Vec<ContractAddressResponse>> {
    contracts
        .into_iter()
        .map(|contract| query_contract_address(deps, contract))
        .collect::<StdResult<Vec<_>>>()
}

fn query_all_contract_addresses(
    deps: Deps,
    start_after: Option<MarsContract>,
    limit: Option<u32>,
) -> StdResult<Vec<ContractAddressResponse>> {
    let start = start_after.map(MarsAddressKey::from).map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    CONTRACTS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            Ok(ContractAddressResponse {
                contract: k.try_into()?,
                address: v,
            })
        })
        .collect()
}

fn query_gov_address(deps: Deps, gov: MarsGov) -> StdResult<GovAddressResponse> {
    Ok(GovAddressResponse {
        gov,
        address: GOVERNANCE.load(deps.storage, gov.into())?,
    })
}

fn query_gov_addresses(deps: Deps, gov: Vec<MarsGov>) -> StdResult<Vec<GovAddressResponse>> {
    gov.into_iter().map(|gov| query_gov_address(deps, gov)).collect::<StdResult<Vec<_>>>()
}

fn query_all_gov_addresses(
    deps: Deps,
    start_after: Option<MarsGov>,
    limit: Option<u32>,
) -> StdResult<Vec<GovAddressResponse>> {
    let start = start_after.map(MarsAddressKey::from).map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    GOVERNANCE
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            Ok(GovAddressResponse {
                gov: k.try_into()?,
                address: v,
            })
        })
        .collect()
}
