use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
    StdResult,
};
use cw2::set_contract_version;

use rover::{ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::execute::try_create_credit_account;
use crate::instantiate::{
    instantiate_nft_contract, store_config, store_nft_contract_addr,
    NFT_CONTRACT_INSTANTIATE_REPLY_ID,
};
use crate::query::{
    query_allowed_assets, query_allowed_vaults, query_nft_contract_addr, query_owner,
};

const CONTRACT_NAME: &str = "crates.io:rover-credit-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    store_config(deps, &msg)?;
    let sub_message = instantiate_nft_contract(msg.nft_contract_code_id, msg.owner, env)?;
    Ok(Response::new()
        .add_submessage(sub_message)
        .add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::CreateCreditAccount {} => try_create_credit_account(deps, info.sender),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _: Env, reply: Reply) -> StdResult<Response> {
    match reply.id {
        id if id == NFT_CONTRACT_INSTANTIATE_REPLY_ID => store_nft_contract_addr(deps, reply),
        id => Err(StdError::generic_err(format!("invalid reply id: {}", id))),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Owner {} => to_binary(&query_owner(deps)?),
        QueryMsg::AllowedVaults { start_after, limit } => {
            to_binary(&query_allowed_vaults(deps, start_after, limit)?)
        }
        QueryMsg::AllowedAssets { start_after, limit } => {
            to_binary(&query_allowed_assets(deps, start_after, limit)?)
        }
        QueryMsg::CreditAccountNftAddress {} => to_binary(&query_nft_contract_addr(deps)?),
    }
}
