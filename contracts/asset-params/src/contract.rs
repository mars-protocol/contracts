use cosmwasm_std::{Binary, Decimal, Deps, DepsMut, entry_point, Env, MessageInfo, Response, to_binary};
use cw2::set_contract_version;
use mars_owner::OwnerInit::SetInitialOwner;
use mars_utils::error::ValidationError;
use mars_utils::helpers::decimal_param_le_one;
use crate::execute::{init_asset, init_or_update_vault, update_asset, update_close_factor, update_owner};
use crate::query::{query_asset_params, query_config, query_vault_config};
use crate::error::{ContractError, ContractResult};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{CLOSE_FACTOR, OWNER};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    validate_cf(msg.close_factor,"close-factor")?;

    CLOSE_FACTOR.save(deps.storage, &msg.close_factor)?;

    OWNER.initialize(
        deps.storage,
        deps.api,
        SetInitialOwner {
            owner: msg.owner.clone(),
        },
    )?;
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::UpdateOwner(update) => update_owner(deps, info, update),
        ExecuteMsg::UpdateCloseFactor {
            close_factor,
        } => update_close_factor(deps, info, close_factor),
        ExecuteMsg::InitAsset {
            denom,
            params,
        } => init_asset(deps, info, denom, params),
        ExecuteMsg::UpdateAsset {
            denom,
            params,
        } => update_asset(deps, info, denom, params),
        ExecuteMsg::InitOrUpdateVault {
            address,
            config,
        } => init_or_update_vault(deps, info, address, config),
    }
}

#[entry_point]
pub fn query(deps: Deps, msg: QueryMsg) -> Result<Binary, ContractError> {
    let res = match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::AssetParamsResponse {
            denom,
        } => to_binary(&query_asset_params(deps, denom)?),
        QueryMsg::VaultConfigsResponse {
            address,
        } => to_binary(&query_vault_config(deps, address)?),
    };
    res.map_err(Into::into)
}

pub fn validate_cf(param_value: Decimal, param_name: &str) -> Result<(), ValidationError> {
    if !param_value.le(&Decimal::one()) {
        Err(ValidationError::InvalidParam {
            param_name: param_name.to_string(),
            invalid_value: param_value.to_string(),
            predicate: "<= 1".to_string(),
        })
    } else {
        Ok(())
    }
}