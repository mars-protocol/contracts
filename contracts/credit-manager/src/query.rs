use std::convert::TryFrom;

use cosmwasm_std::{Addr, Deps, Order, StdResult, Uint128};
use cw_asset::{AssetInfo, AssetInfoKey, AssetInfoUnchecked, AssetUnchecked};
use cw_storage_plus::{Bound, Map};
use rover::msg::query::{ConfigResponse, PositionResponse, TotalDebtSharesResponse};

use crate::state::{
    NftTokenId, ACCOUNT_NFT, ALLOWED_ASSETS, ALLOWED_VAULTS, ASSETS, DEBT_SHARES, OWNER, RED_BANK,
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
        assets: get_asset_list(deps, token_id, ASSETS)?,
        debt_shares: get_asset_list(deps, token_id, DEBT_SHARES)?,
    })
}

fn get_asset_list(
    deps: Deps,
    token_id: &str,
    asset_amount_map: Map<(NftTokenId, AssetInfoKey), Uint128>,
) -> StdResult<Vec<AssetUnchecked>> {
    asset_amount_map
        .prefix(token_id)
        .range(deps.storage, None, None, Order::Ascending)
        .into_iter()
        .map(|res| {
            let (asset_info_key, amount) = res?;
            let info_unchecked = AssetInfoUnchecked::try_from(asset_info_key)?;
            Ok(AssetUnchecked::new(info_unchecked, amount.u128()))
        })
        .collect()
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

/// NOTE: This implementation of the query function assumes the map `ALLOWED_ASSETS` only saves `true`.
/// If an asset is to be removed from the whitelist, the map must remove the corresponding key, instead
/// of setting the value to `false`.
pub fn query_allowed_assets(
    deps: Deps,
    start_after: Option<AssetInfoUnchecked>,
    limit: Option<u32>,
) -> StdResult<Vec<AssetInfoUnchecked>> {
    let info: AssetInfo;
    let start = match &start_after {
        Some(unchecked) => {
            info = unchecked.check(deps.api, None)?;
            Some(Bound::exclusive(AssetInfoKey::from(info)))
        }
        None => None,
    };

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    ALLOWED_ASSETS
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?
        .into_iter()
        .map(AssetInfoUnchecked::try_from)
        .collect()
}

pub fn query_total_debt_shares(
    deps: Deps,
    unchecked_asset_info: AssetInfoUnchecked,
) -> StdResult<TotalDebtSharesResponse> {
    let asset_info = unchecked_asset_info.check(deps.api, None)?;
    let total_debt = TOTAL_DEBT_SHARES.load(deps.storage, asset_info.clone().into())?;
    Ok(TotalDebtSharesResponse(AssetUnchecked::new(
        asset_info, total_debt,
    )))
}
