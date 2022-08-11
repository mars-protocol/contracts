use cosmwasm_std::{Addr, Decimal, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

use rover::error::ContractResult;
use rover::msg::query::{
    CoinBalanceResponseItem, CoinShares, CoinValue, ConfigResponse, DebtSharesValue,
    PositionResponse, SharesResponseItem,
};
use rover::{Denom, NftTokenId};

use crate::state::{
    ACCOUNT_NFT, ALLOWED_COINS, ALLOWED_VAULTS, COIN_BALANCES, DEBT_SHARES, ORACLE, OWNER,
    RED_BANK, TOTAL_DEBT_SHARES,
};

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    Ok(ConfigResponse {
        owner: OWNER.load(deps.storage)?.into(),
        account_nft: ACCOUNT_NFT
            .may_load(deps.storage)?
            .map(|addr| addr.to_string()),
        red_bank: RED_BANK.load(deps.storage)?.address().into(),
        oracle: ORACLE.load(deps.storage)?.address().into(),
    })
}

pub fn query_position(
    deps: Deps,
    env: &Env,
    token_id: NftTokenId,
) -> ContractResult<PositionResponse> {
    let coin_asset_values = get_coin_balances_values(deps, token_id)?;
    let debt_shares_values = get_debt_shares_values(deps, env, token_id)?;

    Ok(PositionResponse {
        token_id: token_id.to_string(),
        coins: coin_asset_values,
        debt_shares: debt_shares_values,
    })
}

pub fn query_all_assets(
    deps: Deps,
    start_after: Option<(String, String)>,
    limit: Option<u32>,
) -> StdResult<Vec<CoinBalanceResponseItem>> {
    let start = start_after
        .as_ref()
        .map(|(token_id, denom)| Bound::exclusive((token_id.as_str(), denom.as_str())));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    Ok(COIN_BALANCES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?
        .iter()
        .map(|((token_id, denom), amount)| CoinBalanceResponseItem {
            token_id: token_id.to_string(),
            denom: denom.to_string(),
            amount: *amount,
        })
        .collect())
}

fn get_debt_shares_values(
    deps: Deps,
    env: &Env,
    token_id: NftTokenId,
) -> ContractResult<Vec<DebtSharesValue>> {
    let oracle = ORACLE.load(deps.storage)?;

    DEBT_SHARES
        .prefix(token_id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|res| {
            let (denom, shares) = res?;
            // total shares of debt issued for denom
            let total_debt_shares = TOTAL_DEBT_SHARES
                .load(deps.storage, &denom)
                .unwrap_or(Uint128::zero());

            // total rover debt amount in Redbank for asset
            let red_bank = RED_BANK.load(deps.storage)?;
            let total_debt_amount =
                red_bank.query_debt(&deps.querier, &env.contract.address, &denom)?;

            // amount of debt for token's position
            // NOTE: Given the nature of integers, the debt is rounded down. This means that the
            //       remaining share owners will take a small hit of the remainder.
            let owed = total_debt_amount.checked_multiply_ratio(shares, total_debt_shares)?;
            let owed_dec = Decimal::from_atomics(owed, 0)?;

            // debt value of token's position
            let coin_price = oracle.query_price(&deps.querier, &denom)?;
            let position_debt_value = coin_price.checked_mul(owed_dec)?;

            Ok(DebtSharesValue {
                total_value: position_debt_value,
                denom,
                shares,
            })
        })
        .collect()
}

fn get_coin_balances_values(deps: Deps, token_id: &str) -> ContractResult<Vec<CoinValue>> {
    let oracle = ORACLE.load(deps.storage)?;
    COIN_BALANCES
        .prefix(token_id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (denom, amount) = item?;
            let price = oracle.query_price(&deps.querier, &denom)?;
            let decimal_amount = Decimal::from_atomics(amount, 0)?;
            let value = price.checked_mul(decimal_amount)?;
            Ok(CoinValue {
                denom,
                amount,
                price,
                value,
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
    let addr: Addr;
    let start = match &start_after {
        Some(addr_str) => {
            addr = deps.api.addr_validate(addr_str)?;
            Some(Bound::exclusive(&addr))
        }
        None => None,
    };

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    ALLOWED_VAULTS
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            let addr = res?;
            Ok(addr.to_string())
        })
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
