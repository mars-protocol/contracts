use cosmwasm_std::{Coin, Deps, Env, Order, StdResult};
use cw_paginate::paginate_map;
use cw_storage_plus::Bound;
use mars_rover::{
    adapters::vault::{VaultBase, VaultPosition, VaultPositionValue, VaultUnchecked},
    error::ContractResult,
    msg::query::{
        CoinBalanceResponseItem, ConfigResponse, DebtAmount, DebtShares, LentAmount, LentShares,
        Positions, SharesResponseItem, VaultPositionResponseItem, VaultUtilizationResponse,
    },
};

use crate::{
    state::{
        ACCOUNT_NFT, COIN_BALANCES, DEBT_SHARES, HEALTH_CONTRACT, LENT_SHARES,
        MAX_UNLOCKING_POSITIONS, ORACLE, OWNER, PARAMS, RED_BANK, REWARDS_COLLECTOR, SWAPPER,
        TOTAL_DEBT_SHARES, TOTAL_LENT_SHARES, VAULT_POSITIONS, ZAPPER,
    },
    utils::{debt_shares_to_amount, lent_shares_to_amount},
    vault::vault_utilization_in_deposit_cap_denom,
};

pub fn query_config(deps: Deps) -> ContractResult<ConfigResponse> {
    Ok(ConfigResponse {
        ownership: OWNER.query(deps.storage)?,
        account_nft: ACCOUNT_NFT.may_load(deps.storage)?.map(|a| a.address().into()),
        red_bank: RED_BANK.load(deps.storage)?.address().into(),
        oracle: ORACLE.load(deps.storage)?.address().into(),
        params: PARAMS.load(deps.storage)?.address().into(),
        max_unlocking_positions: MAX_UNLOCKING_POSITIONS.load(deps.storage)?,
        swapper: SWAPPER.load(deps.storage)?.address().into(),
        zapper: ZAPPER.load(deps.storage)?.address().into(),
        health_contract: HEALTH_CONTRACT.load(deps.storage)?.address().into(),
        rewards_collector: REWARDS_COLLECTOR.may_load(deps.storage)?.map(|rc| rc.address),
    })
}

pub fn query_positions(deps: Deps, env: &Env, account_id: &str) -> ContractResult<Positions> {
    Ok(Positions {
        account_id: account_id.to_string(),
        deposits: query_coin_balances(deps, account_id)?,
        debts: query_debt_amounts(deps, env, account_id)?,
        lends: query_lent_amounts(deps, env, account_id)?,
        vaults: query_vault_positions(deps, account_id)?,
    })
}

pub fn query_all_coin_balances(
    deps: Deps,
    start_after: Option<(String, String)>,
    limit: Option<u32>,
) -> StdResult<Vec<CoinBalanceResponseItem>> {
    let start = start_after
        .as_ref()
        .map(|(account_id, denom)| Bound::exclusive((account_id.as_str(), denom.as_str())));
    paginate_map(&COIN_BALANCES, deps.storage, start, limit, |(account_id, denom), amount| {
        Ok(CoinBalanceResponseItem {
            account_id,
            denom,
            amount,
        })
    })
}

fn query_lent_amounts(deps: Deps, env: &Env, account_id: &str) -> ContractResult<Vec<LentAmount>> {
    LENT_SHARES
        .prefix(account_id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|res| {
            let (denom, shares) = res?;
            let coin = lent_shares_to_amount(deps, &env.contract.address, &denom, shares)?;
            Ok(LentAmount {
                denom,
                shares,
                amount: coin.amount,
            })
        })
        .collect()
}

fn query_debt_amounts(deps: Deps, env: &Env, account_id: &str) -> ContractResult<Vec<DebtAmount>> {
    DEBT_SHARES
        .prefix(account_id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|res| {
            let (denom, shares) = res?;
            let coin = debt_shares_to_amount(deps, &env.contract.address, &denom, shares)?;
            Ok(DebtAmount {
                denom,
                shares,
                amount: coin.amount,
            })
        })
        .collect()
}

pub fn query_coin_balances(deps: Deps, account_id: &str) -> ContractResult<Vec<Coin>> {
    COIN_BALANCES
        .prefix(account_id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (denom, amount) = item?;
            Ok(Coin {
                denom,
                amount,
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
        .map(|(account_id, denom)| Bound::exclusive((account_id.as_str(), denom.as_str())));
    paginate_map(&DEBT_SHARES, deps.storage, start, limit, |(account_id, denom), shares| {
        Ok(SharesResponseItem {
            account_id,
            denom,
            shares,
        })
    })
}

pub fn query_all_lent_shares(
    deps: Deps,
    start_after: Option<(String, String)>,
    limit: Option<u32>,
) -> StdResult<Vec<SharesResponseItem>> {
    let start = start_after
        .as_ref()
        .map(|(account_id, denom)| Bound::exclusive((account_id.as_str(), denom.as_str())));
    paginate_map(&LENT_SHARES, deps.storage, start, limit, |(account_id, denom), shares| {
        Ok(SharesResponseItem {
            account_id,
            denom,
            shares,
        })
    })
}

pub fn query_vault_utilization(
    deps: Deps,
    env: Env,
    unchecked: VaultUnchecked,
) -> ContractResult<VaultUtilizationResponse> {
    let vault = unchecked.check(deps.api)?;
    Ok(VaultUtilizationResponse {
        vault: vault.clone().into(),
        utilization: vault_utilization_in_deposit_cap_denom(&deps, &vault, &env.contract.address)?,
    })
}

pub fn query_vault_positions(deps: Deps, account_id: &str) -> ContractResult<Vec<VaultPosition>> {
    VAULT_POSITIONS
        .prefix(account_id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|res| {
            let (addr, position) = res?;
            Ok(VaultPosition {
                vault: VaultBase::new(addr),
                amount: position,
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
        Some((account_id, unchecked)) => {
            let addr = deps.api.addr_validate(unchecked)?;
            Some(Bound::exclusive((account_id.as_str(), addr)))
        }
        None => None,
    };
    paginate_map(&VAULT_POSITIONS, deps.storage, start, limit, |(account_id, addr), amount| {
        Ok(VaultPositionResponseItem {
            account_id,
            position: VaultPosition {
                vault: VaultBase::new(addr),
                amount,
            },
        })
    })
}

pub fn query_total_debt_shares(deps: Deps, denom: &str) -> StdResult<DebtShares> {
    let shares = TOTAL_DEBT_SHARES.load(deps.storage, denom)?;
    Ok(DebtShares {
        denom: denom.to_string(),
        shares,
    })
}

pub fn query_total_lent_shares(deps: Deps, denom: &str) -> StdResult<LentShares> {
    let shares = TOTAL_LENT_SHARES.load(deps.storage, denom)?;
    Ok(LentShares {
        denom: denom.to_string(),
        shares,
    })
}

pub fn query_all_total_debt_shares(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<DebtShares>> {
    let start = start_after.as_ref().map(|denom| Bound::exclusive(denom.as_str()));
    paginate_map(&TOTAL_DEBT_SHARES, deps.storage, start, limit, |denom, shares| {
        Ok(DebtShares {
            denom,
            shares,
        })
    })
}

pub fn query_all_total_lent_shares(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<LentShares>> {
    let start = start_after.as_ref().map(|denom| Bound::exclusive(denom.as_str()));
    paginate_map(&TOTAL_LENT_SHARES, deps.storage, start, limit, |denom, shares| {
        Ok(LentShares {
            denom,
            shares,
        })
    })
}

pub fn query_vault_position_value(
    deps: Deps,
    vault_position: VaultPosition,
) -> StdResult<VaultPositionValue> {
    let oracle = ORACLE.load(deps.storage)?;
    vault_position.query_values(&deps.querier, &oracle)
}
