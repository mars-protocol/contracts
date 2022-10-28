use cosmwasm_std::{Coin, Deps, Order, StdError, StdResult, Storage, Uint128};
use cw_utils::Duration;

use cosmos_vault_standard::extensions::lockup::Lockup;
use cosmos_vault_standard::msg::{AssetsResponse, VaultInfo};

use crate::error::ContractError::NotLockingVault;
use crate::error::ContractResult;
use crate::state::{COIN_BALANCE, LOCKUPS, LOCKUP_TIME, TOTAL_VAULT_SHARES, VAULT_TOKEN_DENOM};

pub fn query_coins_for_shares(
    storage: &dyn Storage,
    shares: Uint128,
) -> ContractResult<AssetsResponse> {
    let total_shares_opt = TOTAL_VAULT_SHARES.may_load(storage)?;
    let balance = COIN_BALANCE.load(storage)?;
    match total_shares_opt {
        Some(total_vault_shares) if !total_vault_shares.is_zero() => Ok(AssetsResponse {
            coin: Coin {
                denom: balance.denom,
                amount: balance.amount.multiply_ratio(shares, total_vault_shares),
            },
        }),
        _ => Ok(AssetsResponse { coin: balance }),
    }
}

pub fn query_vault_info(deps: Deps) -> ContractResult<VaultInfo> {
    let req_denom = COIN_BALANCE.load(deps.storage)?.denom;
    let vault_token_denom = VAULT_TOKEN_DENOM.load(deps.storage)?;
    Ok(VaultInfo {
        req_denom,
        vault_token_denom,
    })
}

pub fn query_lockup_duration(deps: Deps) -> ContractResult<Duration> {
    let res = LOCKUP_TIME.load(deps.storage)?.ok_or(NotLockingVault)?;
    Ok(res)
}

pub fn query_lockup(deps: Deps, id: u64) -> ContractResult<Lockup> {
    Ok(LOCKUPS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?
        .into_iter()
        .flat_map(|(_, positions)| positions)
        .find(|p| p.id == id)
        .ok_or_else(|| StdError::generic_err("Id not found"))?)
}

pub fn query_lockups(deps: Deps, addr: String) -> ContractResult<Vec<Lockup>> {
    let addr = deps.api.addr_validate(addr.as_str())?;
    let res = LOCKUPS.load(deps.storage, addr)?;
    Ok(res)
}

pub fn query_vault_token_supply(storage: &dyn Storage) -> ContractResult<Uint128> {
    let amount_issued = TOTAL_VAULT_SHARES
        .may_load(storage)?
        .unwrap_or(Uint128::zero());
    Ok(amount_issued)
}
