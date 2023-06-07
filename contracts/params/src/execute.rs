use cosmwasm_std::{Decimal, DepsMut, MessageInfo, Response};
use mars_utils::error::ValidationError;

use crate::{
    error::ContractResult,
    msg::{AssetParamsUpdate, VaultConfigUpdate},
    state::{ASSET_PARAMS, MAX_CLOSE_FACTOR, OWNER, VAULT_CONFIGS},
};

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn update_max_close_factor(
    deps: DepsMut,
    info: MessageInfo,
    max_close_factor: Decimal,
) -> ContractResult<Response> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    assert_mcf(max_close_factor)?;
    MAX_CLOSE_FACTOR.save(deps.storage, &max_close_factor)?;

    let response = Response::new()
        .add_attribute("action", "update_max_close_factor")
        .add_attribute("value", max_close_factor.to_string());

    Ok(response)
}

pub fn update_asset_params(
    deps: DepsMut,
    info: MessageInfo,
    update: AssetParamsUpdate,
) -> ContractResult<Response> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let mut response = Response::new().add_attribute("action", "update_asset_param");

    match update {
        AssetParamsUpdate::AddOrUpdate {
            params: unchecked,
        } => {
            let params = unchecked.check(deps.api)?;

            ASSET_PARAMS.save(deps.storage, &params.denom, &params)?;
            response = response
                .add_attribute("action_type", "add_or_update")
                .add_attribute("denom", params.denom);
        }
    }

    Ok(response)
}

pub fn update_vault_config(
    deps: DepsMut,
    info: MessageInfo,
    update: VaultConfigUpdate,
) -> ContractResult<Response> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let mut response = Response::new().add_attribute("action", "update_vault_config");

    match update {
        VaultConfigUpdate::AddOrUpdate {
            config,
        } => {
            let checked = config.check(deps.api)?;
            VAULT_CONFIGS.save(deps.storage, &checked.addr, &checked)?;
            response = response
                .add_attribute("action_type", "add_or_update")
                .add_attribute("addr", checked.addr);
        }
        VaultConfigUpdate::Remove {
            addr,
        } => {
            let checked = deps.api.addr_validate(&addr)?;
            VAULT_CONFIGS.remove(deps.storage, &checked);
            response = response.add_attribute("action_type", "remove").add_attribute("addr", addr);
        }
    }

    Ok(response)
}

pub fn assert_mcf(param_value: Decimal) -> Result<(), ValidationError> {
    if !param_value.le(&Decimal::one()) {
        Err(ValidationError::InvalidParam {
            param_name: "max-close-factor".to_string(),
            invalid_value: "max-close-factor".to_string(),
            predicate: "<= 1".to_string(),
        })
    } else {
        Ok(())
    }
}

/// liquidation_threshold should be greater than or equal to max_loan_to_value
pub fn assert_lqt_gt_max_ltv(
    max_ltv: Decimal,
    liq_threshold: Decimal,
) -> Result<(), ValidationError> {
    if liq_threshold <= max_ltv {
        return Err(ValidationError::InvalidParam {
            param_name: "liquidation_threshold".to_string(),
            invalid_value: liq_threshold.to_string(),
            predicate: format!("> {} (max LTV)", max_ltv),
        });
    }
    Ok(())
}

pub fn assert_hls_lqt_gt_max_ltv(
    max_ltv: Decimal,
    liq_threshold: Decimal,
) -> Result<(), ValidationError> {
    if liq_threshold <= max_ltv {
        return Err(ValidationError::InvalidParam {
            param_name: "hls_liquidation_threshold".to_string(),
            invalid_value: liq_threshold.to_string(),
            predicate: format!("> {} (hls max LTV)", max_ltv),
        });
    }
    Ok(())
}
