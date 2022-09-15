use cosmwasm_std::{Coin, Deps, Order, StdResult, Storage, Uint128};

use rover::msg::vault::VaultInfo;

use crate::state::{ASSETS, LOCKUP_TIME, LP_TOKEN_DENOM, TOTAL_VAULT_SHARES};

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
    Ok(VaultInfo {
        coins: get_all_vault_coins(deps.storage)?,
        lockup: LOCKUP_TIME.load(deps.storage)?,
        token_denom: LP_TOKEN_DENOM.load(deps.storage)?,
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
