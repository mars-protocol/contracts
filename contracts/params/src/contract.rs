#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;
use mars_owner::OwnerInit::SetInitialOwner;

use crate::{
    emergency_powers::{disable_borrowing, disallow_coin, set_zero_deposit_cap, set_zero_max_ltv},
    error::ContractResult,
    execute::{assert_thf, update_asset_params, update_target_health_factor, update_vault_config},
    msg::{
        CmEmergencyUpdate, EmergencyUpdate, ExecuteMsg, InstantiateMsg, QueryMsg,
        RedBankEmergencyUpdate,
    },
    query::{query_all_asset_params, query_all_vault_configs, query_vault_config},
    state::{ASSET_PARAMS, OWNER, TARGET_HEALTH_FACTOR},
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _: Env,
    _: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    OWNER.initialize(
        deps.storage,
        deps.api,
        SetInitialOwner {
            owner: msg.owner,
        },
    )?;

    assert_thf(msg.target_health_factor)?;
    TARGET_HEALTH_FACTOR.save(deps.storage, &msg.target_health_factor)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::UpdateOwner(update) => Ok(OWNER.update(deps, info, update)?),
        ExecuteMsg::UpdateAssetParams(update) => update_asset_params(deps, info, update),
        ExecuteMsg::UpdateTargetHealthFactor(mcf) => update_target_health_factor(deps, info, mcf),
        ExecuteMsg::UpdateVaultConfig(update) => update_vault_config(deps, info, update),
        ExecuteMsg::EmergencyUpdate(update) => match update {
            EmergencyUpdate::RedBank(rb_u) => match rb_u {
                RedBankEmergencyUpdate::DisableBorrowing(denom) => {
                    disable_borrowing(deps, info, &denom)
                }
            },
            EmergencyUpdate::CreditManager(rv_u) => match rv_u {
                CmEmergencyUpdate::DisallowCoin(denom) => disallow_coin(deps, info, &denom),
                CmEmergencyUpdate::SetZeroMaxLtvOnVault(v) => set_zero_max_ltv(deps, info, &v),
                CmEmergencyUpdate::SetZeroDepositCapOnVault(v) => {
                    set_zero_deposit_cap(deps, info, &v)
                }
            },
        },
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _: Env, msg: QueryMsg) -> ContractResult<Binary> {
    let res = match msg {
        QueryMsg::Owner {} => to_binary(&OWNER.query(deps.storage)?),
        QueryMsg::AssetParams {
            denom,
        } => to_binary(&ASSET_PARAMS.load(deps.storage, &denom)?),
        QueryMsg::AllAssetParams {
            start_after,
            limit,
        } => to_binary(&query_all_asset_params(deps, start_after, limit)?),
        QueryMsg::VaultConfig {
            address,
        } => to_binary(&query_vault_config(deps, &address)?),
        QueryMsg::AllVaultConfigs {
            start_after,
            limit,
        } => to_binary(&query_all_vault_configs(deps, start_after, limit)?),
        QueryMsg::TargetHealthFactor {} => to_binary(&TARGET_HEALTH_FACTOR.load(deps.storage)?),
    };
    res.map_err(Into::into)
}
