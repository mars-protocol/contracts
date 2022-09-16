use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct InstantiateMsg {
    pub coins: Vec<CoinMarketInfo>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct CoinMarketInfo {
    pub denom: String,
    pub max_ltv: Decimal,
    pub liquidation_threshold: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Borrow {
        coin: Coin,
        recipient: Option<String>,
    },
    Repay {
        denom: String,
        on_behalf_of: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(UserAssetDebtResponse)]
    UserAssetDebt { user_address: String, denom: String },
    #[returns(mars_outpost::red_bank::Market)]
    Market { denom: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct UserAssetDebtResponse {
    pub denom: String,
    pub amount: Uint128,
}
