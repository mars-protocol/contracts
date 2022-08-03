use std::fmt;

use cosmwasm_std::{Decimal, QuerierWrapper, StdError, StdResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use mars_oracle_base::{ContractResult, PriceSource};

use osmo_bindings::OsmosisQuery;

use crate::helpers;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OsmosisPriceSource {
    /// Returns a fixed value;
    Fixed {
        price: Decimal,
    },
    /// Osmosis spot price quoted in OSMO
    ///
    /// NOTE: `pool_id` must point to an Osmosis pool consists of the asset of interest and OSMO
    Spot {
        pool_id: u64,
    },
    /// Osmosis liquidity token
    LiquidityToken {
        pool_id: u64,
    },
}

impl fmt::Display for OsmosisPriceSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let label = match self {
            OsmosisPriceSource::Fixed {
                price,
            } => format!("fixed:{}", price),
            OsmosisPriceSource::Spot {
                pool_id,
            } => format!("spot:{}", pool_id),
            OsmosisPriceSource::LiquidityToken {
                pool_id,
            } => format!("liquidity_token:{}", pool_id),
        };
        write!(f, "{}", label)
    }
}

impl PriceSource<OsmosisQuery> for OsmosisPriceSource {
    fn validate(
        &self,
        querier: &QuerierWrapper<OsmosisQuery>,
        denom: impl Into<String>,
    ) -> ContractResult<()> {
        match self {
            OsmosisPriceSource::Fixed {
                ..
            } => Ok(()),
            OsmosisPriceSource::Spot {
                pool_id,
            } => helpers::assert_osmosis_pool_assets(querier, *pool_id, &denom.into()),
            OsmosisPriceSource::LiquidityToken {
                ..
            } => Ok(()),
        }
    }

    fn query_price(
        &self,
        querier: &QuerierWrapper<OsmosisQuery>,
        denom: impl Into<String>,
    ) -> StdResult<Decimal> {
        match self {
            OsmosisPriceSource::Fixed {
                price,
            } => Ok(*price),
            OsmosisPriceSource::Spot {
                pool_id,
            } => helpers::query_osmosis_spot_price(querier, *pool_id, &denom.into()),
            OsmosisPriceSource::LiquidityToken {
                ..
            } => Err(StdError::generic_err("Unimplemented")),
        }
    }
}
