use cosmwasm_std::{Coin, Deps, Order, StdResult};
use cw_storage_plus::Bound;

use rover::msg::query::{
    AssetResponseItem, CoinShares, ConfigResponse, PositionResponse, SharesResponseItem,
};
use rover::Denom;

use crate::state::{
    ACCOUNT_NFT, ALLOWED_COINS, ALLOWED_VAULTS, ASSETS, DEBT_SHARES, OWNER, RED_BANK,
    TOTAL_DEBT_SHARES,
};

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    Ok(ConfigResponse {
        owner: OWNER.load(deps.storage)?.into(),
        account_nft: ACCOUNT_NFT
            .may_load(deps.storage)?
            .map(|addr| addr.to_string()),
        red_bank: RED_BANK.load(deps.storage)?.0.into(),
    })
}

pub fn query_position(deps: Deps, token_id: &str) -> StdResult<PositionResponse> {
    Ok(PositionResponse {
        token_id: token_id.to_string(),
        assets: get_assets(deps, token_id)?,
        debt_shares: get_debt_shares(deps, token_id)?,
    })
}

fn get_assets(deps: Deps, token_id: &str) -> StdResult<Vec<Coin>> {
    ASSETS
        .prefix(token_id)
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?
        .iter()
        .map(|(denom, amount)| {
            Ok(Coin {
                denom: denom.clone(),
                amount: *amount,
            })
        })
        .collect()
}

pub fn query_all_assets(
    deps: Deps,
    start_after: Option<(String, String)>,
    limit: Option<u32>,
) -> StdResult<Vec<AssetResponseItem>> {
    let start = start_after
        .as_ref()
        .map(|(token_id, denom)| Bound::exclusive((token_id.as_str(), denom.as_str())));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    Ok(ASSETS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?
        .iter()
        .map(|((token_id, denom), amount)| AssetResponseItem {
            token_id: token_id.to_string(),
            denom: denom.to_string(),
            amount: *amount,
        })
        .collect())
}

fn get_debt_shares(deps: Deps, token_id: &str) -> StdResult<Vec<CoinShares>> {
    DEBT_SHARES
        .prefix(token_id)
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?
        .iter()
        .map(|(denom, shares)| {
            Ok(CoinShares {
                denom: denom.clone(),
                shares: *shares,
            })
        })
        .collect()
}

pub fn query_all_debt_shares(
    deps: Deps,
    start_after: Option<(String, String)>,
    limit: Option<u32>,
) -> StdResult<Vec<SharesResponseItem>> {
    let start = start_after
        .as_ref()
        .map(|(token_id, denom)| Bound::exclusive((token_id.as_str(), denom.as_str())));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    Ok(DEBT_SHARES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?
        .iter()
        .map(|((token_id, denom), shares)| SharesResponseItem {
            token_id: token_id.to_string(),
            denom: denom.to_string(),
            shares: *shares,
        })
        .collect())
}

/// NOTE: This implementation of the query function assumes the map `ALLOWED_VAULTS` only saves `true`.
/// If a vault is to be removed from the whitelist, the map must remove the corresponding key, instead
/// of setting the value to `false`.
pub fn query_allowed_vaults(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<String>> {
    let start = match &start_after {
        Some(addr_str) => {
            let addr = deps.api.addr_validate(addr_str)?;
            Some(Bound::exclusive(addr))
        }
        None => None,
    };

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    ALLOWED_VAULTS
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|vault_addr| vault_addr.to_string()))
        .collect()
}

/// NOTE: This implementation of the query function assumes the map `ALLOWED_COINS` only saves `true`.
/// If a coin is to be removed from the whitelist, the map must remove the corresponding key, instead
/// of setting the value to `false`.
pub fn query_allowed_coins(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<String>> {
    let start = start_after
        .as_ref()
        .map(|denom| Bound::exclusive(denom.as_str()));

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    ALLOWED_COINS
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()
}

pub fn query_total_debt_shares(deps: Deps, denom: Denom) -> StdResult<CoinShares> {
    let shares = TOTAL_DEBT_SHARES.load(deps.storage, denom)?;
    Ok(CoinShares {
        denom: denom.to_string(),
        shares,
    })
}

pub fn query_all_total_debt_shares(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<CoinShares>> {
    let start = start_after
        .as_ref()
        .map(|denom| Bound::exclusive(denom.as_str()));

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    Ok(TOTAL_DEBT_SHARES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?
        .iter()
        .map(|(denom, shares)| CoinShares {
            denom: denom.to_string(),
            shares: *shares,
        })
        .collect())
}
