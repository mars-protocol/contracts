use cosmwasm_std::{Coin, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InstantiateMsg {
    pub coins: Vec<DenomWithLTV>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct DenomWithLTV {
    pub denom: String,
    pub max_ltv: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Borrow {
        coin: Coin,
        recipient: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    UserAssetDebt { user_address: String, denom: String },
    Market { denom: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserAssetDebtResponse {
    pub denom: String,
    pub amount: Uint128,
}

// Schema reference: https://github.com/mars-protocol/mars-core/blob/master/packages/mars-core/src/red_bank/mod.rs#L47
// TODO: After mars-core bumps to the next version https://crates.io/crates/mars-core (currently 1.0.0)
//       should update this mock to return MarsDecimal:  https://github.com/mars-protocol/mars-core/blob/master/packages/mars-core/src/math/decimal.rs
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Market {
    pub max_loan_to_value: Decimal,
}
