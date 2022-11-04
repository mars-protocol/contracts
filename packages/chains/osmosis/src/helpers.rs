use std::str::FromStr;

use cosmwasm_std::{Decimal, Empty, QuerierWrapper, QueryRequest, StdResult};

use osmosis_std::shim::Timestamp;
use osmosis_std::types::cosmos::base::v1beta1::Coin;
use osmosis_std::types::osmosis::gamm::v1beta1::{
    GammQuerier, PoolAsset, PoolParams, QueryPoolRequest,
};
use osmosis_std::types::osmosis::twap::v1beta1::TwapQuerier;

use serde::{Deserialize, Serialize};

// NOTE: Use custom Pool (`id` type as String) due to problem with json (de)serialization discrepancy between go and rust side.
// https://github.com/osmosis-labs/osmosis-rust/issues/42
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Pool {
    pub id: String,
    pub address: String,
    pub pool_params: Option<PoolParams>,
    pub future_pool_governor: String,
    pub pool_assets: Vec<PoolAsset>,
    pub total_shares: Option<Coin>,
    pub total_weight: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct QueryPoolResponse {
    pub pool: Pool,
}

/// Query an Osmosis pool's coin depths and the supply of of liquidity token
pub fn query_pool(querier: &QuerierWrapper, pool_id: u64) -> StdResult<Pool> {
    let req: QueryRequest<Empty> = QueryPoolRequest { pool_id }.into();
    let res: QueryPoolResponse = querier.query(&req)?;
    Ok(res.pool)
}

pub fn has_denom(denom: &str, pool_assets: &[PoolAsset]) -> bool {
    pool_assets
        .iter()
        .flat_map(|asset| &asset.token)
        .any(|coin| coin.denom == denom)
}

/// Query the spot price of a coin, denominated in OSMO
pub fn query_spot_price(
    querier: &QuerierWrapper,
    pool_id: u64,
    base_denom: &str,
    quote_denom: &str,
) -> StdResult<Decimal> {
    // NOTE: Currency pair consists of base and quote asset (base/quote). Spot query has it swapped.
    // For example:
    // if we want to check the price ATOM/OSMO then we pass base_asset = OSMO, quote_asset = ATOM
    let spot_price_res = GammQuerier::new(querier).spot_price(
        pool_id,
        quote_denom.to_string(),
        base_denom.to_string(),
    )?;
    let price = Decimal::from_str(&spot_price_res.spot_price)?;
    Ok(price)
}

/// Query the twap price of a coin, denominated in OSMO.
/// `start_time` must be within 48 hours of current block time.
pub fn query_twap_price(
    querier: &QuerierWrapper,
    pool_id: u64,
    base_denom: &str,
    quote_denom: &str,
    start_time: u64,
) -> StdResult<Decimal> {
    let arithmetic_twap_res = TwapQuerier::new(querier).arithmetic_twap_to_now(
        pool_id,
        base_denom.to_string(),
        quote_denom.to_string(),
        Some(Timestamp {
            seconds: start_time as i64,
            nanos: 0,
        }),
    )?;
    let price = Decimal::from_str(&arithmetic_twap_res.arithmetic_twap)?;
    Ok(price)
}
