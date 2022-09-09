use std::any::type_name;

use cosmwasm_std::{Decimal, QuerierWrapper, QueryRequest, StdError, StdResult};

use osmo_bindings::{
    ArithmeticTwapToNowResponse, OsmosisQuery, PoolStateResponse, SpotPriceResponse,
};
use osmosis_std::types::osmosis::gamm::v1beta1::{GammQuerier, Pool};
use prost::{DecodeError, Message};

use mars_oracle_base::{ContractError, ContractResult};
use mars_outpost::error::MarsError;

/// Assert the Osmosis pool indicated by `pool_id` contains exactly two assets, and they are OSMO and `denom`
pub fn assert_osmosis_pool_assets(
    querier: &QuerierWrapper<OsmosisQuery>,
    pool_id: u64,
    denom: &str,
    base_denom: &str,
) -> ContractResult<()> {
    let pool = query_osmosis_pool(querier, pool_id)?;

    if pool.assets.len() != 2 {
        return Err(ContractError::InvalidPriceSource {
            reason: format!(
                "expecting pool {} to contain exactly two coins; found {}",
                pool_id,
                pool.assets.len()
            ),
        });
    }

    if !pool.has_denom(base_denom) {
        return Err(ContractError::InvalidPriceSource {
            reason: format!("pool {} does not contain the base denom {}", pool_id, base_denom),
        });
    }

    if !pool.has_denom(denom) {
        return Err(ContractError::InvalidPriceSource {
            reason: format!("pool {} does not contain {}", pool_id, denom),
        });
    }

    Ok(())
}

pub fn assert_osmosis_xyk_pool(
    querier: &QuerierWrapper<OsmosisQuery>,
    pool_id: u64,
) -> ContractResult<()> {
    let pool_res = GammQuerier::new(querier).pool(pool_id)?;
    let pool_type = type_name::<Pool>();
    let pool = pool_res.pool.ok_or_else(|| StdError::not_found(pool_type))?;
    let pool_res: Result<Pool, DecodeError> = Message::decode(pool.value.as_slice());
    let pool = pool_res.map_err(|_| MarsError::Deserialize {
        target_type: pool_type.to_string(),
    })?;

    // NOTE: It is safe because we execute `assert_osmosis_pool_assets` before
    if pool.pool_assets[0].weight != pool.pool_assets[1].weight {
        return Err(ContractError::InvalidPriceSource {
            reason: format!("assets in pool {} do not have equal weights", pool_id),
        });
    }
    Ok(())
}

/// Query the spot price of a coin, denominated in OSMO
pub fn query_osmosis_spot_price(
    querier: &QuerierWrapper<OsmosisQuery>,
    pool_id: u64,
    denom: &str,
    base_denom: &str,
) -> StdResult<Decimal> {
    let query = OsmosisQuery::spot_price(pool_id, denom, base_denom);
    let res: SpotPriceResponse = querier.query(&QueryRequest::Custom(query))?;
    Ok(res.price)
}

/// Query an Osmosis pool's coin depths and the supply of of liquidity token
pub fn query_osmosis_pool(
    querier: &QuerierWrapper<OsmosisQuery>,
    pool_id: u64,
) -> StdResult<PoolStateResponse> {
    querier.query(&QueryRequest::Custom(OsmosisQuery::PoolState {
        id: pool_id,
    }))
}

/// Query the twap price of a coin, denominated in OSMO.
/// `start_time` must be within 48 hours of current block time.
pub fn query_osmosis_twap_price(
    querier: &QuerierWrapper<OsmosisQuery>,
    pool_id: u64,
    denom: &str,
    base_denom: &str,
    start_time: u64,
) -> StdResult<Decimal> {
    // NOTE: quote_asset_denom in TWAP is base_denom (OSMO)
    let query = OsmosisQuery::arithmetic_twap_to_now(pool_id, base_denom, denom, start_time as i64);
    let res: ArithmeticTwapToNowResponse = querier.query(&QueryRequest::Custom(query))?;
    Ok(res.twap)
}
