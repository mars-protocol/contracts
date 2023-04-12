use cosmwasm_std::{entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;
use mars_owner::OwnerInit::SetInitialOwner;

use crate::{
    emergency_powers::{disable_borrowing, disallow_coin, set_zero_deposit_cap, set_zero_max_ltv},
    error::ContractResult,
    execute::{assert_mcf, update_asset_params, update_max_close_factor, update_vault_config},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    query::{query_all_asset_params, query_all_vault_configs, query_vault_config},
    state::{ASSET_PARAMS, MAX_CLOSE_FACTOR, OWNER},
    types::{EmergencyUpdate, RedBankEmergencyUpdate, RoverEmergencyUpdate},
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
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

    assert_mcf(msg.max_close_factor)?;
    MAX_CLOSE_FACTOR.save(deps.storage, &msg.max_close_factor)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::UpdateOwner(update) => Ok(OWNER.update(deps, info, update)?),
        ExecuteMsg::UpdateAssetParams(update) => update_asset_params(deps, info, update),
        ExecuteMsg::UpdateMaxCloseFactor(mcf) => update_max_close_factor(deps, info, mcf),
        ExecuteMsg::UpdateVaultConfig(update) => update_vault_config(deps, info, update),
        ExecuteMsg::EmergencyUpdate(update) => match update {
            EmergencyUpdate::RedBank(rb_u) => match rb_u {
                RedBankEmergencyUpdate::DisableBorrowing(denom) => {
                    disable_borrowing(deps, info, &denom)
                }
            },
            EmergencyUpdate::Rover(rv_u) => match rv_u {
                RoverEmergencyUpdate::DisallowCoin(denom) => disallow_coin(deps, info, &denom),
                RoverEmergencyUpdate::SetZeroMaxLtvOnVault(v) => set_zero_max_ltv(deps, info, &v),
                RoverEmergencyUpdate::SetZeroDepositCapOnVault(v) => {
                    set_zero_deposit_cap(deps, info, &v)
                }
            },
        },
    }
}

#[entry_point]
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
        QueryMsg::MaxCloseFactor {} => to_binary(&MAX_CLOSE_FACTOR.load(deps.storage)?),
    };
    res.map_err(Into::into)
}
