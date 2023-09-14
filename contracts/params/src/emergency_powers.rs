use cosmwasm_std::{Decimal, DepsMut, MessageInfo, Response, Uint128};

use crate::{
    error::ContractError,
    state::{ASSET_PARAMS, OWNER, VAULT_CONFIGS},
};

pub fn disable_borrowing(
    deps: DepsMut,
    info: MessageInfo,
    denom: &str,
) -> Result<Response, ContractError> {
    OWNER.assert_emergency_owner(deps.storage, &info.sender)?;

    let mut params = ASSET_PARAMS.load(deps.storage, denom)?;
    params.red_bank.borrow_enabled = false;
    ASSET_PARAMS.save(deps.storage, denom, &params)?;

    let response = Response::new()
        .add_attribute("action", "emergency_disable_borrowing")
        .add_attribute("denom", denom.to_string());

    Ok(response)
}

pub fn disallow_coin(
    deps: DepsMut,
    info: MessageInfo,
    denom: &str,
) -> Result<Response, ContractError> {
    OWNER.assert_emergency_owner(deps.storage, &info.sender)?;

    let mut params = ASSET_PARAMS.load(deps.storage, denom)?;
    params.credit_manager.whitelisted = false;
    ASSET_PARAMS.save(deps.storage, denom, &params)?;

    let response = Response::new()
        .add_attribute("action", "emergency_disallow_coin")
        .add_attribute("denom", denom.to_string());

    Ok(response)
}

pub fn set_zero_max_ltv(
    deps: DepsMut,
    info: MessageInfo,
    vault: &str,
) -> Result<Response, ContractError> {
    OWNER.assert_emergency_owner(deps.storage, &info.sender)?;

    let vault_addr = deps.api.addr_validate(vault)?;

    let mut config = VAULT_CONFIGS.load(deps.storage, &vault_addr)?;
    config.max_loan_to_value = Decimal::zero();
    VAULT_CONFIGS.save(deps.storage, &vault_addr, &config)?;

    let response = Response::new()
        .add_attribute("action", "emergency_set_zero_max_ltv")
        .add_attribute("vault", vault.to_string());

    Ok(response)
}

pub fn set_zero_deposit_cap(
    deps: DepsMut,
    info: MessageInfo,
    vault: &str,
) -> Result<Response, ContractError> {
    OWNER.assert_emergency_owner(deps.storage, &info.sender)?;

    let vault_addr = deps.api.addr_validate(vault)?;

    let mut config = VAULT_CONFIGS.load(deps.storage, &vault_addr)?;
    config.deposit_cap.amount = Uint128::zero();
    VAULT_CONFIGS.save(deps.storage, &vault_addr, &config)?;

    let response = Response::new()
        .add_attribute("action", "emergency_set_zero_deposit_cap")
        .add_attribute("vault", vault.to_string());

    Ok(response)
}
