use cosmwasm_std::{Coin, Deps, Order, StdError, StdResult, Storage, Uint128};

use rover::msg::vault::{UnlockingPosition, VaultInfo};

use crate::state::{ASSETS, LOCKUP_TIME, LP_TOKEN_DENOM, TOTAL_VAULT_SHARES, UNLOCKING_COINS};

pub fn query_coins_for_shares(storage: &dyn Storage, shares: Uint128) -> StdResult<Vec<Coin>> {
    let total_shares_opt = TOTAL_VAULT_SHARES.may_load(storage)?;
    match total_shares_opt {
        None => Ok(vec![]),
        Some(total_vault_shares) => {
            let all_vault_coins = get_all_vault_coins(storage)?;
            let coins_for_shares = all_vault_coins
                .iter()
                .map(|asset| Coin {
                    denom: asset.clone().denom,
                    amount: asset.amount.multiply_ratio(shares, total_vault_shares),
                })
                .collect::<Vec<Coin>>();
            Ok(coins_for_shares)
        }
    }
}

pub fn query_vault_info(deps: Deps) -> StdResult<VaultInfo> {
    let all_coins = get_all_vault_coins(deps.storage)?;
    let accepted_denoms = all_coins.iter().map(|c| c.denom.clone()).collect();
    Ok(VaultInfo {
        accepts: vec![accepted_denoms],
        lockup: LOCKUP_TIME.load(deps.storage)?,
        vault_coin_denom: LP_TOKEN_DENOM.load(deps.storage)?,
    })
}

pub fn get_all_vault_coins(storage: &dyn Storage) -> StdResult<Vec<Coin>> {
    ASSETS
        .range(storage, None, None, Order::Ascending)
        .map(|res| {
            let (denom, amount) = res?;
            Ok(Coin { denom, amount })
        })
        .collect()
}

pub fn query_unlocking_position(deps: Deps, id: Uint128) -> StdResult<UnlockingPosition> {
    UNLOCKING_COINS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?
        .into_iter()
        .flat_map(|(_, positions)| positions)
        .find(|p| p.id == id)
        .ok_or_else(|| StdError::generic_err("Id not found"))
}

pub fn query_unlocking_positions(deps: Deps, addr: String) -> StdResult<Vec<UnlockingPosition>> {
    let addr = deps.api.addr_validate(addr.as_str())?;
    let res = UNLOCKING_COINS.load(deps.storage, addr)?;
    Ok(res)
}

pub fn query_vault_coins_issued(storage: &dyn Storage) -> StdResult<Uint128> {
    TOTAL_VAULT_SHARES.load(storage)
}
