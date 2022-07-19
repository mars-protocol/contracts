pub mod contract;
pub mod error;
pub mod state;

use cosmwasm_std::Decimal;
pub use mars_outpost::oracle::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PriceSource {
    /// Returns a fixed value;
    Fixed { price: Decimal },

    /// Osmosis spot price quoted in OSMO
    ///
    /// NOTE: `pool_id` must point to an Osmosis pool consists of the asset of interest and OSMO
    Spot {
        /// Pool id
        pool_id: u64,
    },

    /// Osmosis liquidity token
    LiquidityToken {
        /// Pool id
        pool_id: u64,
    },
}

impl fmt::Display for PriceSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let label = match self {
            PriceSource::Fixed { .. } => "fixed",
            PriceSource::Spot { .. } => "spot",
            PriceSource::LiquidityToken { .. } => "liquidity_token",
        };
        write!(f, "{}", label)
    }
}
