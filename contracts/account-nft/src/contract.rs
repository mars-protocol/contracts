use std::convert::TryInto;

use cosmwasm_std::{
    entry_point, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use cw721_base::{ContractError, Cw721Contract, Extension, InstantiateMsg, QueryMsg};

use crate::execute::{try_mint, try_update_owner};
use crate::msg::ExecuteMsg;

// Extending CW721 base contract
pub type Parent<'a> = Cw721Contract<'a, Extension, Empty>;

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
    msg: ExecuteMsg<Extension>,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Mint(mint_msg) => try_mint(deps, env, info, mint_msg),
        ExecuteMsg::UpdateOwner { new_owner } => try_update_owner(deps, new_owner),
        _ => Parent::default().execute(deps, env, info, msg.try_into()?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    Parent::default().query(deps, env, msg)
}
