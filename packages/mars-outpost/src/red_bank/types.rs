use cosmwasm_std::{Addr, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::MarsError;
use crate::helpers::decimal_param_le_one;

/// Global configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    /// Contract owner
    pub owner: Addr,
    /// Address provider returns addresses for all protocol contracts
    pub address_provider_address: Addr,
    /// maToken code id used to instantiate new tokens
    pub ma_token_code_id: u64,
    /// Maximum percentage of outstanding debt that can be covered by a liquidator
    pub close_factor: Decimal,
}

impl Config {
    pub fn validate(&self) -> Result<(), MarsError> {
        decimal_param_le_one(self.close_factor, "close_factor")?;
        Ok(())
    }
}

/// RedBank global state
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct GlobalState {
    /// Market count
    pub market_count: u32,
}

/// Data for individual users
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct User {
    /// bits representing borrowed assets. 1 on the corresponding bit means asset is
    /// being borrowed
    pub borrowed_assets: Uint128,
    /// bits representing collateral assets. 1 on the corresponding bit means asset is
    /// being used as collateral
    pub collateral_assets: Uint128,
}

impl Default for User {
    fn default() -> Self {
        User {
            borrowed_assets: Uint128::zero(),
            collateral_assets: Uint128::zero(),
        }
    }
}

/// Debt for each asset and user
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Debt {
    /// Scaled debt amount
    pub amount_scaled: Uint128,

    /// Marker for uncollateralized debt
    pub uncollateralized: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserHealthStatus {
    NotBorrowing,
    Borrowing(Decimal),
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: Addr,
    pub address_provider_address: Addr,
    pub ma_token_code_id: u64,
    pub market_count: u32,
    pub close_factor: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct UserDebtResponse {
    pub debts: Vec<UserAssetDebtResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct UserAssetDebtResponse {
    /// Asset denom
    pub denom: String,
    /// Scaled debt amount stored in contract state
    pub amount_scaled: Uint128,
    /// Underlying asset amount that is actually owed at the current block
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct UserCollateralResponse {
    pub collateral: Vec<UserAssetCollateralResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct UserAssetCollateralResponse {
    /// Asset denom
    pub denom: String,
    /// Wether the user is using asset as collateral or not
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct UserPositionResponse {
    pub total_collateral_in_base_asset: Uint128,
    pub total_debt_in_base_asset: Uint128,
    /// Total debt minus the uncollateralized debt
    pub total_collateralized_debt_in_base_asset: Uint128,
    pub max_debt_in_base_asset: Uint128,
    pub weighted_liquidation_threshold_in_base_asset: Uint128,
    pub health_status: UserHealthStatus,
}
