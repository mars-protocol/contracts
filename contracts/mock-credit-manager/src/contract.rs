#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use mars_rover::msg::QueryMsg;

use crate::{
    execute::{set_allowed_coins, set_position_response, set_vault_config},
    msg::{ExecuteMsg, InstantiateMsg},
    query::{query_allowed_coins, query_config, query_positions, query_vault_config},
    state::{ALLOWED_COINS, CONFIG},
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    CONFIG.save(deps.storage, &msg.config)?;
    ALLOWED_COINS.save(deps.storage, &vec![])?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::SetPositionsResponse {
            account_id,
            positions,
        } => set_position_response(deps, account_id, positions),
        ExecuteMsg::SetAllowedCoins(allowed_coins) => set_allowed_coins(deps, allowed_coins),
        ExecuteMsg::SetVaultConfig {
            address,
            config,
        } => set_vault_config(deps, &address, config),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Positions {
            account_id,
        } => to_binary(&query_positions(deps, account_id)?),
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::AllowedCoins {
            ..
        } => to_binary(&query_allowed_coins(deps)?),
        QueryMsg::VaultConfig {
            vault,
        } => to_binary(&query_vault_config(deps, vault)?),
        _ => unimplemented!("query msg not supported"),
    }
}
