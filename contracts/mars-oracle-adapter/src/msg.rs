use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal};
use rover::adapters::{Oracle, OracleUnchecked};

#[cw_serde]
pub struct CoinPrice {
    pub denom: String,
    pub price: Decimal,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub oracle: OracleUnchecked,
    pub vault_pricing: Vec<VaultPricingInfo>,
    pub owner: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig { new_config: ConfigUpdates },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// If denom is vault coin, will retrieve priceable underlying before querying oracle
    #[returns(mars_outpost::oracle::PriceResponse)]
    Price { denom: String },

    /// Converts vault coin to the mars-oracle accepted priceable coins
    #[returns(Vec<Coin>)]
    PriceableUnderlying { coin: Coin },

    #[returns(ConfigResponse)]
    Config {},

    #[returns(VaultPricingInfo)]
    PricingInfo { denom: String },

    #[returns(Vec<VaultPricingInfo>)]
    AllPricingInfo {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: Addr,
    pub oracle: Oracle,
}

#[cw_serde]
#[derive(Default)]
pub struct ConfigUpdates {
    pub owner: Option<String>,
    pub oracle: Option<OracleUnchecked>,
    pub vault_pricing: Option<Vec<VaultPricingInfo>>,
}

#[cw_serde]
pub struct VaultPricingInfo {
    pub denom: String,
    pub addr: Addr,
    pub method: PricingMethod,
}

#[cw_serde]
pub enum PricingMethod {
    PreviewRedeem,
}
