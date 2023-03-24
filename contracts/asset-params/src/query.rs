use cosmwasm_std::{Addr, Deps};
use crate::state::{ASSET_PARAMS, CLOSE_FACTOR, OWNER, VAULT_CONFIGS};
use crate::error::ContractResult;
use crate::types::{AssetParams, ConfigResponse, VaultConfigs};

pub fn query_config(deps: Deps) -> ContractResult<ConfigResponse> {
    let owner_res = OWNER.query(deps.storage)?;
    Ok(ConfigResponse {
        owner: owner_res.owner,
        proposed_new_owner: owner_res.proposed,
        emergency_owner: owner_res.emergency_owner,
        close_factor: CLOSE_FACTOR.load(deps.storage)?,
    })
}

pub fn query_asset_params(deps: Deps, denom: String) -> ContractResult<AssetParams> {
    let config = ASSET_PARAMS.load(deps.storage, &denom)?;
    Ok(AssetParams {
        reserve_factor: config.reserve_factor,
        max_loan_to_value: config.max_loan_to_value,
        liquidation_threshold: config.liquidation_threshold,
        liquidation_bonus: config.liquidation_bonus,
        interest_rate_model: config.interest_rate_model,
        red_bank_deposit_enabled: config.red_bank_deposit_enabled,
        red_bank_borrow_enabled: config.red_bank_borrow_enabled,
        red_bank_deposit_cap: config.red_bank_deposit_cap,
        rover_whitelisted: config.rover_whitelisted,
        uncollateralized_loan_limit: config.uncollateralized_loan_limit,
    })
}

pub fn query_vault_config(deps: Deps, address: Addr) -> ContractResult<VaultConfigs> {
    let config = VAULT_CONFIGS.load(deps.storage, &address)?;
    Ok(VaultConfigs {
        max_loan_to_value: config.max_loan_to_value,
        liquidation_threshold: config.liquidation_threshold,
        rover_whitelisted: config.rover_whitelisted,
        deposit_cap: config.deposit_cap,
    })
}
