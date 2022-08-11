use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Owner & account nft address. Response type: `ConfigResponse`
    Config,
    /// Whitelisted vaults. Response type: `Vec<String>`
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
    /// The health of the entire position represented by token. Response type: `Health`
    Health { token_id: String },
    /// Enumerate coin balances for all token positions. Response type: `Vec<CoinBalanceResponseItem>`
    /// start_after accepts (token_id, denom)
    AllCoinBalances {
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
pub struct CoinBalanceResponseItem {
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
    pub denom: String,
    pub amount: Uint128,
    pub price: Decimal,
    pub value: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct DebtSharesValue {
    pub denom: String,
    pub shares: Uint128,
    pub total_value: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PositionResponse {
    /// Unique NFT token id that represents the cross-margin account. The owner of this NFT, owns the account.
    pub token_id: String,
    /// All coin balances with its value
    pub coins: Vec<CoinValue>,
    /// All debt positions with its value
    pub debt_shares: Vec<DebtSharesValue>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub owner: String,
    pub account_nft: Option<String>,
    pub red_bank: String,
    pub oracle: String,
}
