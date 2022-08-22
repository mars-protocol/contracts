#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{attr, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use mars_outpost::error::MarsError;
use mars_outpost::helpers::option_string_to_addr;

use mars_outpost::liquidation_filter::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use mars_outpost::liquidation_filter::{Config, Liquidate};

use crate::error::ContractError;
use crate::state::CONFIG;

// INIT

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        address_provider: deps.api.addr_validate(&msg.address_provider)?,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

// HANDLERS

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::LiquidateMany {
            array,
        } => execute_liquidate(deps, env, info, array),
        ExecuteMsg::UpdateConfig {
            owner,
            address_provider,
        } => Ok(execute_update_config(deps, env, info, owner, address_provider)?),
    }
}

pub fn execute_liquidate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _array: Vec<Liquidate>,
) -> Result<Response, ContractError> {
    // only owner can call this
    let config = CONFIG.load(deps.storage)?;
    let owner = config.owner;
    if info.sender != owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    let response = Response::new().add_attributes(vec![attr("action", "set_asset_incentive")]);
    Ok(response)
}

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: Option<String>,
    address_provider: Option<String>,
) -> Result<Response, MarsError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {});
    };

    config.owner = option_string_to_addr(deps.api, owner, config.owner)?;
    config.address_provider =
        option_string_to_addr(deps.api, address_provider, config.address_provider)?;

    CONFIG.save(deps.storage, &config)?;

    let response = Response::new().add_attribute("action", "update_config");

    Ok(response)
}

// QUERIES

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}
