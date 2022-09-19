use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Decimal, Uint128};
use mars_health::health::Health;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::adapters::{VaultPosition, VaultUnchecked};

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Owner & account nft address
    #[returns(ConfigResponse)]
    Config {},
    /// Whitelisted vaults
    #[returns(Vec<VaultUnchecked>)]
    AllowedVaults {
        start_after: Option<VaultUnchecked>,
        limit: Option<u32>,
    },
    /// Whitelisted coins
    #[returns(Vec<String>)]
    AllowedCoins {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// All positions represented by token with value
    #[returns(PositionsWithValueResponse)]
    Positions { account_id: String },
    /// The health of the account represented by token
    #[returns(HealthResponse)]
    Health { account_id: String },
    /// Enumerate coin balances for all token positions; start_after accepts (account_id, denom)
    #[returns(Vec<CoinBalanceResponseItem>)]
    AllCoinBalances {
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    },
    /// Enumerate debt shares for all token positions; start_after accepts (account_id, denom)
    #[returns(Vec<SharesResponseItem>)]
    AllDebtShares {
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    },
    /// Total debt shares issued for Coin
    #[returns(DebtShares)]
    TotalDebtShares(String),
    /// Enumerate total debt shares for all supported coins; start_after accepts denom string
    #[returns(Vec<DebtShares>)]
    AllTotalDebtShares {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Enumerate all vault positions; start_after accepts (account_id, addr)
    #[returns(Vec<VaultPositionResponseItem>)]
    AllVaultPositions {
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    },
    /// Get total vault coin balance in Rover for vault
    #[returns(Uint128)]
    TotalVaultCoinBalance { vault: VaultUnchecked },
    /// Enumerate all total vault coin balances; start_after accepts vault addr
    #[returns(Vec<VaultWithBalance>)]
    AllTotalVaultCoinBalances {
        start_after: Option<VaultUnchecked>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CoinBalanceResponseItem {
    pub account_id: String,
    pub denom: String,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SharesResponseItem {
    pub account_id: String,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Positions {
    pub account_id: String,
    pub coins: Vec<Coin>,
    pub debt: Vec<DebtShares>,
    pub vault_positions: Vec<VaultPositionWithAddr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct VaultPositionResponseItem {
    pub account_id: String,
    pub addr: String,
    pub vault_position: VaultPosition,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct VaultWithBalance {
    pub vault: VaultUnchecked,
    pub balance: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct VaultPositionWithAddr {
    pub addr: String,
    pub position: VaultPosition,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PositionsWithValueResponse {
    /// Unique NFT token id that represents the cross-margin account. The owner of this NFT, owns the account.
    pub account_id: String,
    /// All coin balances value
    pub coins: Vec<CoinValue>,
    /// All debt positions with value
    pub debt: Vec<DebtSharesValue>,
    // TODO: After pricing method is complete, add to response
    /// All vault positions
    pub vault_positions: Vec<VaultPositionWithAddr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub owner: String,
    pub account_nft: Option<String>,
    pub red_bank: String,
    pub oracle: String,
    pub max_liquidation_bonus: Decimal,
    pub max_close_factor: Decimal,
    pub swapper: String,
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
