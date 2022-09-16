use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct CoinPrice {
    pub denom: String,
    pub price: Decimal,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct InstantiateMsg {
    pub coins: Vec<CoinPrice>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // Meant to simulate price changes for tests. Not available in prod.
    ChangePrice(CoinPrice),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(mars_outpost::oracle::PriceResponse)]
    Price { denom: String },
}
