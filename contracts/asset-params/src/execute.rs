use cosmwasm_std::{Addr, Decimal, Env, Uint128};

pub const CONTRACT_NAME: &str = "crates.io:mars-asset-params";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

use crate::state::{ASSET_PARAMS, VAULT_CONFIGS};
use crate::state::{
    CLOSE_FACTOR, OWNER,
};
use cosmwasm_std::{DepsMut, MessageInfo, Response};
use mars_owner::{OwnerError, OwnerUpdate};
use mars_red_bank_types::address_provider::MarsAddressType;
use mars_red_bank_types::error::MarsError;
use mars_red_bank_types::red_bank::{InitOrUpdateAssetParams, Market};
use mars_utils::helpers::validate_native_denom;
use crate::error::{ContractError, ContractResult};
use crate::error::ContractError::InvalidConfig;
use crate::types::{AssetParams, VaultConfigs};

pub fn update_owner(
    deps: DepsMut,
    info: MessageInfo,
    update: OwnerUpdate,
) -> ContractResult<Response> {
    Ok(OWNER.update(deps, info, update)?)
}

pub fn update_close_factor(
    deps: DepsMut,
    info: MessageInfo,
    max_close_factor: Decimal,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    assert_lte_to_one(&max_close_factor)?;

    CLOSE_FACTOR.save(deps.storage, &max_close_factor)?;
    let response = Response::new()
        .add_attribute("key", "max_close_factor")
        .add_attribute("value", max_close_factor.to_string());

    Ok(response)
}

/// Initialize asset if it does not exist.
/// Initialization requires that all params are provided and there is no asset in state.
pub fn init_asset(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
    params: AssetParams,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    validate_native_denom(&denom)?;

    if ASSET_PARAMS.may_load(deps.storage, &denom)?.is_some() {
        return Err(ContractError::AssetAlreadyInitialized {});
    }

    params.validate()?;

    ASSET_PARAMS.save(deps.storage, &denom, &params)?;

    Ok(Response::new().add_attribute("action", "init_asset").add_attribute("denom", denom))
}

/// Update asset with new params.
/// Updating an asset allows you to update just some params.
pub fn update_asset(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
    params: AssetParams,
) -> Result<Response, ContractError> {
    if OWNER.is_owner(deps.storage, &info.sender)? {
        update_asset_by_owner(deps, &denom, params)
    } else if OWNER.is_emergency_owner(deps.storage, &info.sender)? {
        update_asset_by_emergency_owner(deps, &denom, params)
    } else {
        Err(OwnerError::NotOwner {}.into())
    }
}

fn update_asset_by_owner(
    deps: DepsMut,
    denom: &str,
    params: AssetParams,
) -> Result<Response, ContractError> {
    let current_params = ASSET_PARAMS.may_load(deps.storage, denom)?;
    match current_params {
        None => Err(ContractError::AssetNotInitialized {}),
        Some(mut asset) => {

            // need two structs - one optional and one not optional

            // Destructuring a struct’s fields into separate variables in order to force
            // compile error if we add more params
            let AssetParams {
                max_loan_to_value,
                liquidation_threshold,
                liquidation_bonus,
                rover_whitelisted,
                red_bank_deposit_enabled,
                red_bank_borrow_enabled,
                red_bank_deposit_cap,
                interest_rate_model,
                reserve_factor,
                uncollateralized_loan_limit,
            } = params;

            let mut response = Response::new();

            let mut updated_asset = AssetParams {
                max_loan_to_value: max_loan_to_value.unwrap_or(asset.max_loan_to_value),
                liquidation_threshold: liquidation_threshold.unwrap_or(asset.liquidation_threshold),
                liquidation_bonus: liquidation_bonus.unwrap_or(asset.liquidation_bonus),
                rover_whitelisted: rover_whitelisted.unwrap_or(asset.rover_whitelisted),
                red_bank_deposit_enabled: red_bank_deposit_enabled.unwrap_or(asset.red_bank_deposit_enabled),
                red_bank_borrow_enabled: red_bank_borrow_enabled.unwrap_or(asset.red_bank_borrow_enabled),
                red_bank_deposit_cap: red_bank_deposit_cap.unwrap_or(asset.red_bank_deposit_cap),
                interest_rate_model: interest_rate_model.unwrap_or(asset.interest_rate_model),
                reserve_factor: reserve_factor.unwrap_or(asset.reserve_factor),
                uncollateralized_loan_limit: uncollateralized_loan_limit.unwrap_or(asset.uncollateralized_loan_limit),
            };

            ASSET_PARAMS.save(deps.storage, denom, &updated_asset)?;

            Ok(response.add_attribute("action", "update_asset").add_attribute("denom", denom))
        }
    }
}
/// Emergency owner can only DISABLE BORROWING.
fn update_asset_by_emergency_owner(
    deps: DepsMut,
    denom: &str,
    params: AssetParams,
) -> Result<Response, ContractError> {
    if let Some(mut asset) = ASSET_PARAMS.may_load(deps.storage, denom)? {
        match params.red_bank_borrow_enabled {
            Some(borrow_enabled) if !borrow_enabled => {
                asset.red_bank_borrow_enabled = borrow_enabled;
                ASSET_PARAMS.save(deps.storage, denom, &asset)?;

                Ok(Response::new()
                    .add_attribute("action", "emergency_update_asset")
                    .add_attribute("denom", denom))
            }
            _ => ContractError::Unauthorized {}.into(),
        }
    } else {
        ContractError::AssetNotInitialized {}
    }
}

pub fn assert_lte_to_one(dec: &Decimal) -> ContractResult<()> {
    if dec > &Decimal::one() {
        return ContractError(InvalidConfig {
            reason: "value greater than one".to_string(),
        });
    }
    Ok(())
}

pub fn init_or_update_vault(
    deps: DepsMut,
    info: MessageInfo,
    address: Addr,
    config: VaultConfigs,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let VaultConfigs {
        deposit_cap,
        max_loan_to_value,
        liquidation_threshold,
        rover_whitelisted,
    } = config;

    let config_status = VAULT_CONFIGS.may_load(deps.storage, &address)?;
    match config_status {
        None => {
            // All fields should be available
            let available = max_loan_to_value.is_some()
                && deposit_cap.is_some()
                && liquidation_threshold.is_some()
                && rover_whitelisted.is_some();

            if !available {
                return Err(MarsError::InstantiateParamsUnavailable {}.into());
            }
            let mut response = Response::new();
            let new_config = VaultConfigs {
                deposit_cap: deposit_cap.unwrap(),
                max_loan_to_value: max_loan_to_value.unwrap(),
                liquidation_threshold: liquidation_threshold.unwrap(),
                rover_whitelisted: rover_whitelisted.unwrap(),
            };
            VAULT_CONFIGS.save(deps.storage, &address, &new_config)?;
            Ok(response.add_attribute("action", "init_vault").add_attribute("address", address))
        }
        Some(mut config) => {
            let mut updated_config = VaultConfigs {
                deposit_cap: deposit_cap.unwrap_or(config.deposit_cap),
                max_loan_to_value: max_loan_to_value.unwrap_or(config.max_loan_to_value),
                liquidation_threshold: liquidation_threshold.unwrap_or(config.liquidation_threshold),
                rover_whitelisted: rover_whitelisted.unwrap_or(config.rover_whitelisted),
            };

            let mut response = Response::new();

            VAULT_CONFIGS.save(deps.storage, &address, &updated_config)?;

            Ok(response.add_attribute("action", "update_vault_config").add_attribute("address", address))
        }
    }
}

