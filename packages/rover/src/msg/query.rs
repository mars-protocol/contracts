use cosmwasm_std::{Coin, Decimal, Uint128};
use mars_health::health::Health;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
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
    /// All positions represented by token with value. Response type: `PositionsWithValueResponse`
    Positions { token_id: String },
    /// The health of the account represented by token. Response type: `HealthResponse`
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CoinBalanceResponseItem {
    pub token_id: String,
    pub denom: String,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SharesResponseItem {
    pub token_id: String,
    pub denom: String,
    pub shares: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct DebtShares {
    pub denom: String,
    pub shares: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct DebtSharesValue {
    pub denom: String,
    /// number of shares in debt pool
    pub shares: Uint128,
    /// amount of coins
    pub amount: Uint128,
    /// price per coin
    pub price: Decimal,
    /// price * amount
    pub value: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CoinValue {
    pub denom: String,
    pub amount: Uint128,
    pub price: Decimal,
    pub value: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Positions {
    pub token_id: String,
    pub coins: Vec<Coin>,
    pub debt: Vec<DebtShares>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PositionsWithValueResponse {
    /// Unique NFT token id that represents the cross-margin account. The owner of this NFT, owns the account.
    pub token_id: String,
    /// All coin balances value
    pub coins: Vec<CoinValue>,
    /// All debt positions with value
    pub debt: Vec<DebtSharesValue>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub owner: String,
    pub account_nft: Option<String>,
    pub red_bank: String,
    pub oracle: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct HealthResponse {
    pub total_debt_value: Decimal,
    pub total_collateral_value: Decimal,
    pub max_ltv_adjusted_collateral: Decimal,
    pub liquidation_threshold_adjusted_collateral: Decimal,
    pub max_ltv_health_factor: Option<Decimal>,
    pub liquidation_health_factor: Option<Decimal>,
    pub liquidatable: bool,
    pub above_max_ltv: bool,
}

impl From<Health> for HealthResponse {
    fn from(h: Health) -> Self {
        Self {
            total_debt_value: h.total_debt_value,
            total_collateral_value: h.total_collateral_value,
            max_ltv_adjusted_collateral: h.max_ltv_adjusted_collateral,
            liquidation_threshold_adjusted_collateral: h.liquidation_threshold_adjusted_collateral,
            max_ltv_health_factor: h.max_ltv_health_factor,
            liquidation_health_factor: h.liquidation_health_factor,
            liquidatable: h.is_liquidatable(),
            above_max_ltv: h.is_above_max_ltv(),
        }
    }
}
