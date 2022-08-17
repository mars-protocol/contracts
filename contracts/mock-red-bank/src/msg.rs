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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    UserAssetDebt { user_address: String, denom: String },
    Market { denom: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct UserAssetDebtResponse {
    pub denom: String,
    pub amount: Uint128,
}

// Schema reference: https://github.com/mars-protocol/mars-core/blob/master/packages/mars-core/src/red_bank/mod.rs#L47
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Market {
    pub max_loan_to_value: Decimal,
    pub liquidation_threshold: Decimal,
}
