#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;
use mars_owner::OwnerInit::SetInitialOwner;
use mars_rover_health_types::{ConfigResponse, ExecuteMsg, HealthResult, InstantiateMsg, QueryMsg};

use crate::{
    compute::{health_state, health_values},
    state::{CREDIT_MANAGER, OWNER},
    update_config::update_config,
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _: Env,
    _: MessageInfo,
    msg: InstantiateMsg,
) -> HealthResult<Response> {
    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    OWNER.initialize(
        deps.storage,
        deps.api,
        SetInitialOwner {
            owner: msg.owner,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> HealthResult<Response> {
    match msg {
        ExecuteMsg::UpdateOwner(update) => Ok(OWNER.update(deps, info, update)?),
        ExecuteMsg::UpdateConfig {
            credit_manager,
        } => update_config(deps, info, credit_manager),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _: Env, msg: QueryMsg) -> HealthResult<Binary> {
    let res = match msg {
        QueryMsg::HealthValues {
            account_id,
            kind,
        } => to_binary(&health_values(deps, &account_id, kind)?),
        QueryMsg::HealthState {
            account_id,
            kind,
        } => to_binary(&health_state(deps, &account_id, kind)?),
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    };
    res.map_err(Into::into)
}

pub fn query_config(deps: Deps) -> HealthResult<ConfigResponse> {
    let credit_manager = CREDIT_MANAGER.may_load(deps.storage)?.map(Into::into);
    let owner_response = OWNER.query(deps.storage)?;

    Ok(ConfigResponse {
        credit_manager,
        owner_response,
    })
}
