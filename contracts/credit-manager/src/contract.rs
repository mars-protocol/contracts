use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;

use rover::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::deposit::receive_cw20;
use crate::error::ContractError;
use crate::execute::{create_credit_account, dispatch_actions, execute_callback, update_config};
use crate::instantiate::store_config;
use crate::query::{query_allowed_assets, query_allowed_vaults, query_config, query_position};

const CONTRACT_NAME: &str = "crates.io:rover-credit-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    store_config(deps, &msg)?;
    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateCreditAccount {} => create_credit_account(deps, info.sender),
        ExecuteMsg::UpdateConfig { account_nft, owner } => {
            update_config(deps, info, account_nft, owner)
        }
        ExecuteMsg::Callback(callback) => execute_callback(deps, info, env, callback),
        ExecuteMsg::UpdateCreditAccount { token_id, actions } => {
            dispatch_actions(deps, env, info, &token_id, &actions)
        }
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::AllowedVaults { start_after, limit } => {
            to_binary(&query_allowed_vaults(deps, start_after, limit)?)
        }
        QueryMsg::AllowedAssets { start_after, limit } => {
            to_binary(&query_allowed_assets(deps, start_after, limit)?)
        }
        QueryMsg::Position { token_id } => to_binary(&query_position(deps, &token_id)?),
    }
}
