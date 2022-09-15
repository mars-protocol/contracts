use cosmwasm_std::{Addr, Coin, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

use rover::adapters::{Vault, VaultBase, VaultPosition, VaultUnchecked};
use rover::error::ContractResult;
use rover::msg::query::{
    CoinBalanceResponseItem, ConfigResponse, DebtShares, DebtSharesValue, Positions,
    PositionsWithValueResponse, SharesResponseItem, VaultPositionResponseItem,
    VaultPositionWithAddr, VaultWithBalance,
};
use rover::{Denom, NftTokenId};

use crate::state::{
    ACCOUNT_NFT, ALLOWED_COINS, ALLOWED_VAULTS, COIN_BALANCES, DEBT_SHARES, MAX_CLOSE_FACTOR,
    MAX_LIQUIDATION_BONUS, ORACLE, OWNER, RED_BANK, TOTAL_DEBT_SHARES, VAULT_POSITIONS,
};
use crate::utils::{coin_value, debt_shares_to_amount};

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
        max_liquidation_bonus: MAX_LIQUIDATION_BONUS.load(deps.storage)?,
        max_close_factor: MAX_CLOSE_FACTOR.load(deps.storage)?,
    })
}

pub fn query_position(deps: Deps, token_id: NftTokenId) -> ContractResult<Positions> {
    Ok(Positions {
        token_id: token_id.to_string(),
        coins: query_coin_balances(deps, token_id)?,
        debt: query_debt_shares(deps, token_id)?,
        vault_positions: get_vault_positions(deps, token_id)?,
    })
}

pub fn query_position_with_value(
    deps: Deps,
    env: &Env,
    token_id: &str,
) -> ContractResult<PositionsWithValueResponse> {
    let Positions {
        token_id,
        coins,
        debt,
        vault_positions,
    } = query_position(deps, token_id)?;

    let coin_balances_value = coins
        .iter()
        .map(|coin| coin_value(&deps, coin))
        .collect::<ContractResult<Vec<_>>>()?;

    let debt_with_values = debt
        .iter()
        .map(|item| {
            let coin =
                debt_shares_to_amount(deps, &env.contract.address, &item.denom, item.shares)?;
            let cv = coin_value(&deps, &coin)?;
            Ok(DebtSharesValue {
                denom: cv.denom,
                shares: item.shares,
                amount: cv.amount,
                price: cv.price,
                value: cv.value,
            })
        })
        .collect::<ContractResult<Vec<_>>>()?;

    Ok(PositionsWithValueResponse {
        token_id,
        coins: coin_balances_value,
        debt: debt_with_values,
        vault_positions, // TODO: add vault values here
    })
}

pub fn query_all_coin_balances(
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

fn query_debt_shares(deps: Deps, token_id: NftTokenId) -> ContractResult<Vec<DebtShares>> {
    DEBT_SHARES
        .prefix(token_id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|res| {
            let (denom, shares) = res?;
            Ok(DebtShares { denom, shares })
        })
        .collect()
}

fn query_coin_balances(deps: Deps, token_id: &str) -> ContractResult<Vec<Coin>> {
    COIN_BALANCES
        .prefix(token_id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (denom, amount) = item?;
            Ok(Coin { denom, amount })
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

/// NOTE: This implementation of the query function assumes the map `ALLOWED_VAULTS` only saves `Empty`.
/// If a vault is to be removed from the whitelist, the map must remove the corresponding key.
pub fn query_allowed_vaults(
    deps: Deps,
    start_after: Option<VaultUnchecked>,
    limit: Option<u32>,
) -> StdResult<Vec<VaultUnchecked>> {
    let vault: Vault;
    let start = match &start_after {
        Some(unchecked) => {
            vault = unchecked.check(deps.api)?;
            Some(Bound::exclusive(vault.address()))
        }
        None => None,
    };

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    ALLOWED_VAULTS
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            let addr = res?;
            Ok(VaultBase::new(addr.to_string()))
        })
        .collect()
}

fn get_vault_positions(
    deps: Deps,
    token_id: NftTokenId,
) -> ContractResult<Vec<VaultPositionWithAddr>> {
    VAULT_POSITIONS
        .prefix(token_id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|res| {
            let (a, p) = res?;
            Ok(VaultPositionWithAddr {
                addr: a.to_string(),
                position: VaultPosition {
                    unlocked: p.unlocked,
                    locked: p.locked,
                },
            })
        })
        .collect()
}

pub fn query_all_vault_positions(
    deps: Deps,
    start_after: Option<(String, String)>,
    limit: Option<u32>,
) -> StdResult<Vec<VaultPositionResponseItem>> {
    let start = match &start_after {
        Some((token_id, unchecked)) => {
            let addr = deps.api.addr_validate(unchecked)?;
            Some(Bound::exclusive((token_id.as_str(), addr)))
        }
        None => None,
    };

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    Ok(VAULT_POSITIONS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?
        .iter()
        .map(
            |((token_id, addr), vault_position)| VaultPositionResponseItem {
                token_id: token_id.clone(),
                addr: addr.to_string(),
                vault_position: vault_position.clone(),
            },
        )
        .collect())
}

/// NOTE: This implementation of the query function assumes the map `ALLOWED_COINS` only saves `Empty`.
/// If a coin is to be removed from the whitelist, the map must remove the corresponding key.
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

pub fn query_total_debt_shares(deps: Deps, denom: Denom) -> StdResult<DebtShares> {
    let shares = TOTAL_DEBT_SHARES.load(deps.storage, denom)?;
    Ok(DebtShares {
        denom: denom.to_string(),
        shares,
    })
}

pub fn query_all_total_debt_shares(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<DebtShares>> {
    let start = start_after
        .as_ref()
        .map(|denom| Bound::exclusive(denom.as_str()));

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    Ok(TOTAL_DEBT_SHARES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?
        .iter()
        .map(|(denom, shares)| DebtShares {
            denom: denom.to_string(),
            shares: *shares,
        })
        .collect())
}

pub fn query_total_vault_coin_balance(
    deps: Deps,
    unchecked: &VaultUnchecked,
    rover_addr: &Addr,
) -> StdResult<Uint128> {
    let vault = unchecked.check(deps.api)?;
    vault.query_balance(&deps.querier, rover_addr)
}

pub fn query_all_total_vault_coin_balances(
    deps: Deps,
    rover_addr: &Addr,
    start_after: Option<VaultUnchecked>,
    limit: Option<u32>,
) -> StdResult<Vec<VaultWithBalance>> {
    let vault: Vault;
    let start = match &start_after {
        Some(unchecked) => {
            vault = unchecked.check(deps.api)?;
            Some(Bound::exclusive(vault.address()))
        }
        None => None,
    };

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    ALLOWED_VAULTS
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            let addr = res?;
            let unchecked = VaultBase::new(addr.to_string());
            let vault = unchecked.check(deps.api)?;
            let balance = vault.query_balance(&deps.querier, rover_addr)?;
            Ok(VaultWithBalance {
                vault: vault.into(),
                balance,
            })
        })
        .collect()
}
