#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response};
use cw2::set_contract_version;
use mars_owner::OwnerInit::SetInitialOwner;
use mars_types::health::{ConfigResponse, ExecuteMsg, HealthResult, InstantiateMsg, QueryMsg};

use crate::{
    compute::{health_state, health_values},
    migrations,
    state::{CREDIT_MANAGER, OWNER},
    update_config::update_config,
};

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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

    if let Some(cm) = msg.credit_manager {
        let cm = deps.api.addr_validate(&cm)?;
        CREDIT_MANAGER.save(deps.storage, &cm)?;
    }

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
            action,
        } => to_json_binary(&health_values(deps, &account_id, kind, action)?),
        QueryMsg::HealthState {
            account_id,
            kind,
            action,
        } => to_json_binary(&health_state(deps, &account_id, kind, action)?),
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> HealthResult<Response> {
    migrations::v2_1_0::migrate(deps)
}
