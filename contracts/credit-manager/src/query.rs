use cosmwasm_std::{Decimal, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

use rover::msg::query::{
    AssetResponseItem, CoinShares, CoinSharesValue, CoinValue, ConfigResponse, PositionResponse,
    SharesResponseItem,
};
use rover::{ContractResult, Denom, NftTokenId, Shares};

use crate::health::compute_health;
use crate::state::{
    ACCOUNT_NFT, ALLOWED_COINS, ALLOWED_VAULTS, ASSETS, DEBT_SHARES, ORACLE, OWNER, RED_BANK,
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
        oracle: ORACLE.load(deps.storage)?.0.into(),
    })
}

pub fn query_position(
    deps: Deps,
    env: &Env,
    token_id: NftTokenId,
) -> ContractResult<PositionResponse> {
    let assets = get_assets_value(deps, token_id)?;
    let debt_shares = get_debts_value(deps, env, token_id)?;
    let health = compute_health(&deps, &assets, &debt_shares)?;

    Ok(PositionResponse {
        token_id: token_id.to_string(),
        assets,
        debt_shares,
        assets_value: health.assets_value,
        ltv_adjusted_assets_value: health.ltv_adjusted_assets_value,
        debts_value: health.debts_value,
        health_factor: health.health_factor,
        healthy: health.healthy,
    })
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

fn get_debts_value(
    deps: Deps,
    env: &Env,
    token_id: NftTokenId,
) -> ContractResult<Vec<CoinSharesValue>> {
    let oracle = ORACLE.load(deps.storage)?;
    DEBT_SHARES
        .prefix(token_id)
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<(String, Shares)>>>()?
        .iter()
        .map(|(denom, shares)| {
            // proportion of debt this token represents
            let total_debt_shares = TOTAL_DEBT_SHARES
                .load(deps.storage, denom)
                .unwrap_or(Uint128::zero());
            let token_share_ratio = Decimal::checked_from_ratio(*shares, total_debt_shares)?;

            // total rover debt for asset
            let total_debt_amount = RED_BANK.load(deps.storage)?.query_debt(
                &deps.querier,
                &env.contract.address,
                denom,
            )?;
            let total_debt_amount = Decimal::from_atomics(total_debt_amount, 0)?;

            // debt value of token's position
            let price_per_unit = oracle.query_price(&deps.querier, denom)?;
            let position_debt_value = price_per_unit
                .checked_mul(total_debt_amount)?
                .checked_mul(token_share_ratio)?;

            Ok(CoinSharesValue {
                value: position_debt_value,
                price_per_unit,
                denom: denom.clone(),
                shares: *shares,
            })
        })
        .collect()
}

fn get_assets_value(deps: Deps, token_id: &str) -> ContractResult<Vec<CoinValue>> {
    let oracle = ORACLE.load(deps.storage)?;
    ASSETS
        .prefix(token_id)
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<(String, Uint128)>>>()?
        .iter()
        .map(|(denom, amount)| {
            let price_per_unit = oracle.query_price(&deps.querier, denom)?;
            let decimal_amount = Decimal::from_atomics(*amount, 0)?;
            let value = price_per_unit.checked_mul(decimal_amount)?;
            Ok(CoinValue {
                value,
                price_per_unit,
                denom: denom.clone(),
                amount: *amount,
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

    Ok(ALLOWED_VAULTS
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?
        .iter()
        .map(|addr| addr.to_string())
        .collect())
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
