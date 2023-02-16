use cosmwasm_std::{Deps, Order, StdError, StdResult, Storage, Uint128};
use cosmwasm_vault_standard::{extensions::lockup::UnlockingPosition, msg::VaultInfoResponse};
use cw_utils::Duration;

use crate::{
    error::{ContractError::NotLockingVault, ContractResult},
    state::{
        COIN_BALANCE, LOCKUP_TIME, TOTAL_VAULT_SHARES, UNLOCKING_POSITIONS, VAULT_TOKEN_DENOM,
    },
};

pub fn shares_to_base_denom_amount(
    storage: &dyn Storage,
    shares: Uint128,
) -> ContractResult<Uint128> {
    let total_shares = TOTAL_VAULT_SHARES.load(storage)?;
    let balance = COIN_BALANCE.load(storage)?;

    if total_shares.is_zero() {
        Ok(balance.amount)
    } else {
        Ok(balance.amount.multiply_ratio(shares, total_shares))
    }
}

pub fn query_vault_info(deps: Deps) -> ContractResult<VaultInfoResponse> {
    let base_token = COIN_BALANCE.load(deps.storage)?.denom;
    let vault_token = VAULT_TOKEN_DENOM.load(deps.storage)?;
    Ok(VaultInfoResponse {
        base_token,
        vault_token,
    })
}

pub fn query_lockup_duration(deps: Deps) -> ContractResult<Duration> {
    let res = LOCKUP_TIME.load(deps.storage)?.ok_or(NotLockingVault)?;
    Ok(res)
}

pub fn query_unlocking_position(deps: Deps, id: u64) -> ContractResult<UnlockingPosition> {
    Ok(UNLOCKING_POSITIONS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?
        .into_iter()
        .flat_map(|(_, positions)| positions)
        .find(|p| p.id == id)
        .ok_or_else(|| StdError::generic_err("Id not found"))?)
}

pub fn query_unlocking_positions(
    deps: Deps,
    addr: String,
) -> ContractResult<Vec<UnlockingPosition>> {
    let addr = deps.api.addr_validate(addr.as_str())?;
    let res = UNLOCKING_POSITIONS.load(deps.storage, addr)?;
    Ok(res)
}

pub fn query_vault_token_supply(storage: &dyn Storage) -> ContractResult<Uint128> {
    Ok(TOTAL_VAULT_SHARES.may_load(storage)?.unwrap_or(Uint128::zero()))
}
