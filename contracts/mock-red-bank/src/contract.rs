#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use crate::execute::execute_borrow;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::query::{query_debt, query_market};
use crate::state::ASSET_LTV;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    for item in msg.coins {
        ASSET_LTV.save(deps.storage, item.denom, &item.max_ltv)?
    }
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Borrow {
            coin,
            recipient: _recipient,
        } => execute_borrow(deps, info, coin),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::UserAssetDebt {
            user_address,
            denom,
        } => to_binary(&query_debt(deps, user_address, denom)?),
        QueryMsg::Market { denom } => to_binary(&query_market(deps, denom)?),
    }
}
