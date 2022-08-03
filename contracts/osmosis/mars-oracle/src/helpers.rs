use cosmwasm_std::{Decimal, QuerierWrapper, QueryRequest, StdResult};

use osmo_bindings::{OsmosisQuery, PoolStateResponse, SpotPriceResponse};

use mars_oracle_base::{ContractError, ContractResult};

const BASE_DENOM: &str = "uosmo";

/// Assert the Osmosis pool indicated by `pool_id` contains exactly two assets, and they are OSMO and `denom`
pub(crate) fn assert_osmosis_pool_assets(
    querier: &QuerierWrapper<OsmosisQuery>,
    pool_id: u64,
    denom: &str,
) -> ContractResult<()> {
    let pool = query_osmosis_pool(querier, pool_id)?;

    if pool.assets.len() != 2 {
        return Err(ContractError::InvalidPoolId {
            reason: format!(
                "expecting pool {} to contain exactly two coins; found {}",
                pool_id,
                pool.assets.len()
            ),
        });
    }

    if !pool.has_denom(BASE_DENOM) {
        return Err(ContractError::InvalidPoolId {
            reason: format!("pool {} does not contain the base denom {}", pool_id, BASE_DENOM),
        });
    }

    if !pool.has_denom(denom) {
        return Err(ContractError::InvalidPoolId {
            reason: format!("pool {} does not contain {}", pool_id, denom),
        });
    }

    Ok(())
}

/// Query the spot price of a coin, denominated in OSMO
pub(crate) fn query_osmosis_spot_price(
    querier: &QuerierWrapper<OsmosisQuery>,
    pool_id: u64,
    denom: &str,
) -> StdResult<Decimal> {
    let custom_query = OsmosisQuery::spot_price(pool_id, denom, BASE_DENOM);
    let res: SpotPriceResponse = querier.query(&QueryRequest::Custom(custom_query))?;
    Ok(res.price)
}

/// Query an Osmosis pool's coin depths and the supply of of liquidity token
pub(crate) fn query_osmosis_pool(
    querier: &QuerierWrapper<OsmosisQuery>,
    pool_id: u64,
) -> StdResult<PoolStateResponse> {
    querier.query(&QueryRequest::Custom(OsmosisQuery::PoolState {
        id: pool_id,
    }))
}
