use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

use fields_credit_manager::example::{
    ExecuteMsg, InstantiateMsg, QueryMsg, StoredStringResponse,
};

use crate::state::SOME_STRING;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    SOME_STRING.save(deps.storage, &msg.some_string)?;
    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, _env: Env, _: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateItemString {
            str,
        } => try_update_item(deps, str),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetStoredString {} => to_binary(&try_get_stored_str(deps)?),
    }
}

fn try_get_stored_str(deps: Deps) -> StdResult<StoredStringResponse> {
    let str = SOME_STRING.load(deps.storage)?;
    Ok(StoredStringResponse {
        str,
    })
}

fn try_update_item(deps: DepsMut, str: String) -> StdResult<Response> {
    SOME_STRING.save(deps.storage, &str)?;
    Ok(Response::new().add_attribute("method", "UpdateItemString"))
}
