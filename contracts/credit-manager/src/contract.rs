use std::convert::TryFrom;

use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw_asset::AssetInfoUnchecked;

use rover::{AllowListsResponse, ExecuteMsg, InstantiateMsg, OwnerResponse, QueryMsg};

use crate::state::{ALLOWED_ASSETS, ALLOWED_VAULTS, OWNER};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let owner = deps.api.addr_validate(&msg.owner)?;
    OWNER.save(deps.storage, &owner)?;

    msg
        .allowed_vaults
        .iter()
        .try_for_each(|vault| {
            ALLOWED_VAULTS.save(deps.storage, deps.api.addr_validate(vault)?, &true)
        })?;

    msg
        .allowed_assets
        .iter()
        .try_for_each(|info| {
            ALLOWED_ASSETS.save(deps.storage, info.check(deps.api, None)?.into(), &true)
        })?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(_: DepsMut, _env: Env, _: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {}
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetOwner {} => to_binary(&try_get_owner(deps)?),
        QueryMsg::GetAllowLists {} => to_binary(&try_get_allow_lists(deps)?),
    }
}

fn try_get_owner(deps: Deps) -> StdResult<OwnerResponse> {
    let str = OWNER.load(deps.storage)?;
    Ok(OwnerResponse {
        owner: str,
    })
}

fn try_get_allow_lists(deps: Deps) -> StdResult<AllowListsResponse> {
    let vaults = ALLOWED_VAULTS
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    let assets = ALLOWED_ASSETS
        .keys(deps.storage, None, None, Order::Ascending)
        .map(|key| AssetInfoUnchecked::try_from(key?))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(AllowListsResponse {
        vaults,
        assets,
    })
}
