use std::any::type_name;
use std::str::FromStr;

use cosmwasm_std::{Addr, Decimal, QuerierWrapper, StdError, StdResult, Uint128};

use osmosis_std::shim::Timestamp;
use osmosis_std::types::osmosis::gamm::v1beta1::{GammQuerier, Pool, PoolAsset, SwapAmountInRoute};
use osmosis_std::types::osmosis::twap::v1beta1::TwapQuerier;
use prost::DecodeError;

/// Query an Osmosis pool's coin depths and the supply of of liquidity token
pub fn query_pool(querier: &QuerierWrapper, pool_id: u64) -> StdResult<Pool> {
    let res = GammQuerier::new(querier).pool(pool_id)?;
    let pool_type = type_name::<Pool>();
    res.pool
        .ok_or_else(|| StdError::NotFound {
            kind: "pool".to_string(),
        })?
        .try_into()
        .map_err(|e: DecodeError| StdError::ParseErr {
            target_type: pool_type.to_string(),
            msg: e.to_string(),
        })
}

pub fn has_denom(denom: &str, pool_assets: &[PoolAsset]) -> bool {
    pool_assets.iter().flat_map(|asset| &asset.token).any(|coin| coin.denom == denom)
}

/// Query the spot price of a coin, denominated in OSMO
pub fn query_spot_price(
    querier: &QuerierWrapper,
    pool_id: u64,
    denom: &str,
    base_denom: &str,
) -> StdResult<Decimal> {
    let spot_price_res =
        GammQuerier::new(querier).spot_price(pool_id, denom.to_string(), base_denom.to_string())?;
    let price = Decimal::from_str(&spot_price_res.spot_price)?;
    Ok(price)
}

/// Query the twap price of a coin, denominated in OSMO.
/// `start_time` must be within 48 hours of current block time.
pub fn query_twap_price(
    querier: &QuerierWrapper,
    pool_id: u64,
    denom: &str,
    base_denom: &str,
    start_time: u64,
) -> StdResult<Decimal> {
    let arithmetic_twap_res = TwapQuerier::new(querier).arithmetic_twap_to_now(
        pool_id,
        denom.to_string(),
        base_denom.to_string(),
        Some(Timestamp {
            seconds: start_time as i64,
            nanos: 0,
        }),
    )?;
    let price = Decimal::from_str(&arithmetic_twap_res.arithmetic_twap)?;
    Ok(price)
}

/// Estimates how much receives for input amount
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
