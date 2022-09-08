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
    pub address_provider: Addr,
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

// TODO: Once maToken is removed, the scaled collateral amount will be stored in this struct
#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, JsonSchema)]
pub struct Collateral {
    /// Whether this asset is enabled as collateral
    ///
    /// Set to true by default, unless the user explicitly disables it by invoking the
    /// `update_asset_collateral_status` execute method.
    ///
    /// If disabled, the asset will not be subject to liquidation, but will not be considered when
    /// evaluting the user's health factor either.
    pub enabled: bool,
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
    Borrowing {
        max_ltv_hf: Decimal,
        liq_threshold_hf: Decimal,
    },
}

/// User asset settlement
#[derive(Default, Debug)]
pub struct Position {
    pub denom: String,
    pub collateral_amount: Uint128,
    pub debt_amount: Uint128,
    pub uncollateralized_debt: bool,
    pub max_ltv: Decimal,
    pub liquidation_threshold: Decimal,
    pub asset_price: Decimal,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub address_provider: String,
    pub ma_token_code_id: u64,
    pub market_count: u32,
    pub close_factor: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct UncollateralizedLoanLimitResponse {
    /// Asset denom
    pub denom: String,
    /// Uncollateralized loan limit in this asset
    pub limit: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct UserDebtResponse {
    /// Asset denom
    pub denom: String,
    /// Scaled debt amount stored in contract state
    pub amount_scaled: Uint128,
    /// Underlying asset amount that is actually owed at the current block
    pub amount: Uint128,
}

// TODO: In an upcoming PR, we will also include `amount_scaled` and `amount` in this response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct UserCollateralResponse {
    /// Asset denom
    pub denom: String,
    /// Wether the user is using asset as collateral or not
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct UserPositionResponse {
    pub total_collateral_value: Decimal,
    pub total_debt_value: Decimal,
    /// Total debt minus the uncollateralized debt
    pub total_collateralized_debt: Decimal,
    pub weighted_max_ltv_collateral: Decimal,
    pub weighted_liquidation_threshold_collateral: Decimal,
    pub health_status: UserHealthStatus,
}
