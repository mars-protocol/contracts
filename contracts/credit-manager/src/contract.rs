use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw_asset::{AssetInfo, AssetInfoUnchecked};
use std::convert::TryFrom;

use fields::messages::{AllowListsResponse, ExecuteMsg, InstantiateMsg, OwnerResponse, QueryMsg};

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
    store_allow_lists(deps, msg.allowed_vaults, msg.allowed_assets)?;
    Ok(Response::new().add_attribute("method", "instantiate"))
}

fn store_allow_lists(
    deps: DepsMut,
    allowed_vaults: Vec<String>,
    allowed_assets: Vec<AssetInfo>,
) -> StdResult<()> {
    for unverified_addr in &allowed_vaults {
        let addr = deps.api.addr_validate(unverified_addr)?;
        ALLOWED_VAULTS.save(deps.storage, addr, &true)?;
    }

    for denom_or_addr in &allowed_assets {
        match denom_or_addr {
            AssetInfo::Cw20(unverified_addr) => {
                deps.api.addr_validate(unverified_addr.as_str())?;
            }
            _ => {}
        }
        ALLOWED_ASSETS.save(deps.storage, denom_or_addr.into(), &true)?;
    }
    Ok(())
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
        .map(|unchecked| unchecked?.check(deps.api, None))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(AllowListsResponse {
        vaults,
        assets,
    })
}
