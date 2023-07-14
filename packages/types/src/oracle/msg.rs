use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Empty};
use mars_owner::OwnerUpdate;

#[cw_serde]
pub struct InstantiateMsg<C = Empty> {
    /// The contract's owner, who can update config and price sources
    pub owner: String,
    /// The asset in which prices are denominated in
    pub base_denom: String,
    /// Custom init params
    pub custom_init: Option<C>,
}

#[cw_serde]
pub struct Config {
    /// The asset in which prices are denominated in
    pub base_denom: String,
}

#[cw_serde]
pub enum ExecuteMsg<T, C = Empty> {
    /// Specify the price source to be used for a coin
    ///
    /// NOTE: The input parameters for method are chain-specific.
    SetPriceSource {
        denom: String,
        price_source: T,
    },
    /// Remove price source for a coin
    RemovePriceSource {
        denom: String,
    },
    /// Manages admin role state
    UpdateOwner(OwnerUpdate),
    /// Update contract config (only callable by owner)
    UpdateConfig {
        base_denom: Option<String>,
    },
    /// Custom messages defined by the contract
    Custom(C),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Query contract config.
    #[returns(ConfigResponse)]
    Config {},
    /// Query a coin's price source.
    ///
    /// NOTE: The response type of this query is chain-specific.
    #[returns(PriceSourceResponse<String>)]
    PriceSource {
        denom: String,
    },
    /// Enumerate all coins' price sources.
    ///
    /// NOTE: The response type of this query is chain-specific.
    #[returns(Vec<PriceSourceResponse<String>>)]
    PriceSources {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Query a coin's price.
    ///
    /// NOTE: This query may be dependent on block time (e.g. if the price source is TWAP), so may not
    /// work properly with time travel queries on archive nodes.
    #[returns(PriceResponse)]
    Price {
        denom: String,
    },
    /// Enumerate all coins' prices.
    ///
    /// NOTE: This query may be dependent on block time (e.g. if the price source is TWAP), so may not
    /// work properly with time travel queries on archive nodes.
    #[returns(Vec<PriceResponse>)]
    Prices {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    /// The contract's owner
    pub owner: Option<String>,
    /// The contract's proposed owner
    pub proposed_new_owner: Option<String>,
    /// The asset in which prices are denominated in
    pub base_denom: String,
}

#[cw_serde]
pub struct PriceSourceResponse<T> {
    pub denom: String,
    pub price_source: T,
}

#[cw_serde]
pub struct PriceResponse {
    pub denom: String,
    pub price: Decimal,
}

pub mod helpers {
    use cosmwasm_std::{Decimal, QuerierWrapper, StdError, StdResult};

    use super::{PriceResponse, QueryMsg};

    pub fn query_price(
        querier: &QuerierWrapper,
        oracle: impl Into<String>,
        denom: impl Into<String>,
    ) -> StdResult<Decimal> {
        let denom = denom.into();
        let res: PriceResponse = querier
            .query_wasm_smart(
                oracle.into(),
                &QueryMsg::Price {
                    denom: denom.clone(),
                },
            )
            .map_err(|e| {
                StdError::generic_err(format!(
                    "failed to query price for denom: {}. Error: {}",
                    denom, e
                ))
            })?;
        Ok(res.price)
    }
}
