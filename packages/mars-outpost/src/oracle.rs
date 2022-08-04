use cosmwasm_std::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config<T> {
    pub owner: T,
}

pub type InstantiateMsg = Config<String>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg<T> {
    /// Update contract config
    UpdateConfig {
        owner: Option<String>,
    },
    /// Specify the price source to be used for a coin
    ///
    /// NOTE: The input parameters for method are chain-specific.
    SetPriceSource {
        denom: String,
        price_source: T,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Query contract config. Returns `Config<String>`
    Config {},
    /// Query a coin's price source. Returns `PriceSourceResponse`
    ///
    /// NOTE: The response type of this query is chain-specific.
    PriceSource {
        denom: String,
    },
    /// Enumerate all coins' price sources. Returns `Vec<PriceSourceResponse>`
    ///
    /// NOTE: The response type of this query is chain-specific.
    PriceSources {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Query a coin's price; returns `PriceResponse`
    ///
    /// NOTE: This query may be dependent on block time (e.g. if the price source is TWAP), so may not
    /// work properly with time travel queries on archive nodes.
    Price {
        denom: String,
    },
    /// Enumerate all coins' prices. Returns `Vec<PriceResponse>`
    ///
    /// NOTE: This query may be dependent on block time (e.g. if the price source is TWAP), so may not
    /// work properly with time travel queries on archive nodes.
    Prices {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PriceSourceResponse<T> {
    pub denom: String,
    pub price_source: T,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PriceResponse {
    pub denom: String,
    pub price: Decimal,
}

pub mod helpers {
    use super::{PriceResponse, QueryMsg};
    use cosmwasm_std::{Decimal, QuerierWrapper, StdResult};

    pub fn query_price(
        querier: &QuerierWrapper,
        oracle: impl Into<String>,
        denom: impl Into<String>,
    ) -> StdResult<Decimal> {
        let res: PriceResponse = querier.query_wasm_smart(
            oracle.into(),
            &QueryMsg::Price {
                denom: denom.into(),
            },
        )?;
        Ok(res.price)
    }
}
