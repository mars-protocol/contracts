use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Owner & account nft address. Response type: `ConfigResponse`
    Config,
    /// Whitelisted vaults. Response type: `Vec<WithMaxLTV<String>>`
    AllowedVaults {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Whitelisted coins. Response type: `Vec<String>`
    AllowedCoins {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// The entire position represented by token. Response type: `PositionResponse`
    Position { token_id: String },
    /// Enumerate assets for all token positions. Response type: `Vec<AssetResponseItem>`
    /// start_after accepts (token_id, denom)
    AllAssets {
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    },
    /// Enumerate debt shares for all token positions. Response type: `Vec<SharesResponseItem>`
    /// start_after accepts (token_id, denom)
    AllDebtShares {
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    },
    /// Total debt shares issued for Coin. Response type: `CoinShares`
    TotalDebtShares(String),
    /// Enumerate total debt shares for all supported coins. Response type: `Vec<CoinShares>`
    /// start_after accepts denom string
    AllTotalDebtShares {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AssetResponseItem {
    pub token_id: String,
    pub denom: String,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SharesResponseItem {
    pub token_id: String,
    pub denom: String,
    pub shares: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CoinShares {
    pub denom: String,
    pub shares: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CoinValue {
    pub value: Decimal,
    pub price_per_unit: Decimal,
    pub denom: String,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CoinSharesValue {
    pub value: Decimal,
    pub price_per_unit: Decimal,
    pub denom: String,
    pub shares: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PositionResponse {
    pub token_id: String,
    pub assets: Vec<CoinValue>,
    pub debt_shares: Vec<CoinSharesValue>,
    pub assets_value: Decimal,
    pub ltv_adjusted_assets_value: Decimal,
    pub debts_value: Decimal,
    pub health_factor: Option<Decimal>,
    pub healthy: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub owner: String,
    pub account_nft: Option<String>,
    pub red_bank: String,
    pub oracle: String,
}
