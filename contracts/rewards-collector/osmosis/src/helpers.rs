use cosmwasm_std::{Addr, QuerierWrapper, StdError, StdResult, Uint128};
use mars_outpost::error::MarsError;
use osmosis_std::types::osmosis::gamm::v1beta1::{GammQuerier, Pool, PoolAsset, SwapAmountInRoute};
use prost::{DecodeError, Message};
use std::any::type_name;
use std::collections::HashSet;
use std::hash::Hash;
use std::str::FromStr;

/// Build a hashset from array data
pub fn hashset<T: Eq + Clone + Hash>(data: &[T]) -> HashSet<T> {
    data.iter().cloned().collect()
}

/// Query an Osmosis pool's coin depths and the supply of of liquidity token
pub fn query_osmosis_pool(querier: &QuerierWrapper, pool_id: u64) -> StdResult<Pool> {
    let pool_res = GammQuerier::new(querier).pool(pool_id)?;
    let pool_type = type_name::<Pool>();
    let pool = pool_res.pool.ok_or_else(|| StdError::not_found(pool_type))?;
    let pool_res: Result<Pool, DecodeError> = Message::decode(pool.value.as_slice());
    let pool = pool_res.map_err(|_| MarsError::Deserialize {
        target_type: pool_type.to_string(),
    })?;
    Ok(pool)
}

pub fn has_denom(denom: &str, pool_assets: &[PoolAsset]) -> bool {
    pool_assets.iter().flat_map(|asset| &asset.token).any(|coin| coin.denom == denom)
}

pub fn query_estimate_swap_out_amount(
    querier: &QuerierWrapper,
    contract_addr: &Addr,
    pool_id: u64,
    amount: Uint128,
    steps: &[SwapAmountInRoute],
) -> StdResult<Uint128> {
    let exact_amount_in_res = GammQuerier::new(querier).estimate_swap_exact_amount_in(
        contract_addr.to_string(),
        pool_id,
        amount.to_string(),
        steps.to_vec(),
    )?;
    let token_out_amount = Uint128::from_str(&exact_amount_in_res.token_out_amount)?;
    Ok(token_out_amount)
}
