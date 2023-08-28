use std::convert::TryInto;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use cw721_base::Cw721Contract;

use crate::{
    error::ContractError,
    execute::{burn, mint, update_config},
    migrations,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    nft_config::NftConfig,
    query::{query_config, query_next_id},
    state::{CONFIG, NEXT_ID},
};

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Extending CW721 base contract
pub type Parent<'a> = Cw721Contract<'a, Empty, Empty, Empty, Empty>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    NEXT_ID.save(deps.storage, &1)?;

    let health_contract_addr = msg
        .health_contract
        .as_ref()
        .map(|unchecked| deps.api.addr_validate(unchecked))
        .transpose()?;

    CONFIG.save(
        deps.storage,
        &NftConfig {
            max_value_for_burn: msg.max_value_for_burn,
            health_contract_addr,
        },
    )?;

    Parent::default().instantiate(deps, env, info, msg.into())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Mint {
            user,
        } => mint(deps, info, &user),
        ExecuteMsg::UpdateConfig {
            updates,
        } => update_config(deps, info, updates),
        ExecuteMsg::Burn {
            token_id,
        } => burn(deps, env, info, token_id),
        _ => Parent::default().execute(deps, env, info, msg.try_into()?).map_err(Into::into),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::NextId {} => to_binary(&query_next_id(deps)?),
        _ => Parent::default().query(deps, env, msg.try_into()?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
    migrations::v2_0_0::migrate(deps)
}
