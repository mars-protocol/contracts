use std::convert::TryInto;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use cw721_base::{ContractError, Cw721Contract, InstantiateMsg};

use crate::execute::{accept_ownership, mint, propose_new_owner};
use crate::msg::{ExecuteMsg, QueryMsg};
use crate::query::query_proposed_new_owner;

// Extending CW721 base contract
pub type Parent<'a> = Cw721Contract<'a, Empty, Empty, Empty, Empty>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    Parent::default().instantiate(deps, env, info, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Mint { user } => mint(deps, env, info, &user),
        ExecuteMsg::ProposeNewOwner { new_owner } => propose_new_owner(deps, info, &new_owner),
        ExecuteMsg::AcceptOwnership {} => accept_ownership(deps, info),
        _ => Parent::default().execute(deps, env, info, msg.try_into()?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ProposedNewOwner {} => to_binary(&query_proposed_new_owner(deps)?),
        _ => Parent::default().query(deps, env, msg.try_into()?),
    }
}
