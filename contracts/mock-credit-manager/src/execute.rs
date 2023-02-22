use cosmwasm_std::{Addr, DepsMut, Response, StdResult};
use mars_rover::{adapters::vault::VaultConfig, msg::query::Positions};

use crate::state::{ALLOWED_COINS, POSITION_RESPONSES, VAULT_CONFIGS};

pub fn set_position_response(
    deps: DepsMut,
    account_id: String,
    positions: Positions,
) -> StdResult<Response> {
    POSITION_RESPONSES.save(deps.storage, &account_id, &positions)?;
    Ok(Response::new())
}

pub fn set_allowed_coins(deps: DepsMut, coins: Vec<String>) -> StdResult<Response> {
    ALLOWED_COINS.save(deps.storage, &coins)?;
    Ok(Response::new())
}

pub fn set_vault_config(deps: DepsMut, address: &str, config: VaultConfig) -> StdResult<Response> {
    VAULT_CONFIGS.save(deps.storage, &Addr::unchecked(address), &config)?;
    Ok(Response::new())
}
