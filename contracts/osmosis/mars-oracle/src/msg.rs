use std::fmt;

use cosmwasm_std::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PriceSource {
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

impl fmt::Display for PriceSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let label = match self {
            PriceSource::Fixed {
                price,
            } => format!("fixed:{}", price),
            PriceSource::Spot {
                pool_id,
            } => format!("spot:{}", pool_id),
            PriceSource::LiquidityToken {
                pool_id,
            } => format!("liquidity_token:{}", pool_id),
        };
        write!(f, "{}", label)
    }
}

pub type ExecuteMsg = mars_outpost::oracle::ExecuteMsg<PriceSource>;
pub type PriceSourceResponse = mars_outpost::oracle::PriceSourceResponse<PriceSource>;
