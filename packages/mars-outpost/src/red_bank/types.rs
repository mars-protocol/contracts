use cosmwasm_std::{Addr, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::error::MarsError;
use crate::helpers::decimal_param_le_one;
use crate::red_bank;

/// Global configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Contract owner
    pub owner: Addr,
    /// Address provider returns addresses for all protocol contracts
    pub address_provider_address: Addr,
    /// Maximum percentage of outstanding debt that can be covered by a liquidator
    pub close_factor: Decimal,
}

impl Config {
    pub fn validate(&self) -> Result<(), MarsError> {
        decimal_param_le_one(self.close_factor, "close_factor")?;
        Ok(())
    }
}

/// Asset markets
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Market {
    /// Denom of the asset
    pub denom: String,

    /// Max base asset that can be borrowed per "base asset" collateral when using the asset as collateral
    pub max_loan_to_value: Decimal,
    /// Base asset amount in debt position per "base asset" of asset collateral that if surpassed makes the user's position liquidatable.
    pub liquidation_threshold: Decimal,
    /// Bonus amount of collateral liquidator get when repaying user's debt (Will get collateral
    /// from user in an amount equal to debt repayed + bonus)
    pub liquidation_bonus: Decimal,
    /// Portion of the borrow rate that is kept as protocol rewards
    pub reserve_factor: Decimal,

    /// model (params + internal state) that defines how interest rate behaves
    pub interest_rate_model: red_bank::InterestRateModel,

    /// Borrow index (Used to compute borrow interest)
    pub borrow_index: Decimal,
    /// Liquidity index (Used to compute deposit interest)
    pub liquidity_index: Decimal,
    /// Rate charged to borrowers
    pub borrow_rate: Decimal,
    /// Rate paid to depositors
    pub liquidity_rate: Decimal,
    /// Timestamp (seconds) where indexes and where last updated
    pub indexes_last_updated: u64,

    /// Total collateral scaled for the market's currency
    pub collateral_total_scaled: Uint128,
    /// Total debt scaled for the market's currency
    pub debt_total_scaled: Uint128,

    /// If false cannot do any action (deposit/withdraw/borrow/repay/liquidate)
    pub active: bool,
    /// If false cannot deposit
    pub deposit_enabled: bool,
    /// If false cannot borrow
    pub borrow_enabled: bool,
}

impl Market {
    pub fn validate(&self) -> Result<(), MarketError> {
        decimal_param_le_one(self.max_loan_to_value, "max_loan_to_value")?;
        decimal_param_le_one(self.liquidation_threshold, "liquidation_threshold")?;
        decimal_param_le_one(self.liquidation_bonus, "liquidation_bonus")?;

        // liquidation_threshold should be greater than max_loan_to_value
        if self.liquidation_threshold <= self.max_loan_to_value {
            return Err(MarketError::InvalidLiquidationThreshold {
                liquidation_threshold: self.liquidation_threshold,
                max_loan_to_value: self.max_loan_to_value,
            });
        }

        Ok(())
    }
}

impl Default for Market {
    fn default() -> Self {
        let dynamic_ir_model = red_bank::InterestRateModel::Dynamic {
            params: red_bank::DynamicInterestRateModelParams {
                min_borrow_rate: Decimal::zero(),
                max_borrow_rate: Decimal::one(),
                kp_1: Default::default(),
                optimal_utilization_rate: Default::default(),
                kp_augmentation_threshold: Default::default(),
                kp_2: Default::default(),

                update_threshold_txs: 1,
                update_threshold_seconds: 0,
            },
            state: red_bank::DynamicInterestRateModelState {
                txs_since_last_borrow_rate_update: 0,
                borrow_rate_last_updated: 0,
            },
        };

        Market {
            denom: "".to_string(),
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::one(),
            borrow_rate: Default::default(),
            liquidity_rate: Default::default(),
            max_loan_to_value: Default::default(),
            reserve_factor: Default::default(),
            indexes_last_updated: 0,
            collateral_total_scaled: Default::default(),
            debt_total_scaled: Default::default(),
            liquidation_threshold: Decimal::one(),
            liquidation_bonus: Decimal::zero(),
            interest_rate_model: dynamic_ir_model,
            active: true,
            deposit_enabled: true,
            borrow_enabled: true,
        }
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum MarketError {
    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("liquidation_threshold should be greater than max_loan_to_value. liquidation_threshold: {liquidation_threshold:?}, max_loan_to_value: {max_loan_to_value:?}")]
    InvalidLiquidationThreshold {
        liquidation_threshold: Decimal,
        max_loan_to_value: Decimal,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Debt {
    /// Scaled debt amount
    pub amount_scaled: Uint128,
    /// Marker for uncollateralized debt
    pub uncollateralized: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserHealthStatus {
    NotBorrowing,
    Borrowing(Decimal),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Collateral {
    /// Scaled collateral amount
    pub amount_scaled: Uint128,
    /// Whether this collateral is active
    ///
    /// Active collaterals count towards the user's health factor, but is susceptible to
    /// liquidations. On the other hand, inactive collaterals cannot be liquidated, but they don't
    /// add to the user's health factor.
    ///
    /// When making a new fresh deposit, this is set to `true` be default. The user can optionally
    /// invoke the `update_asset_collateral_status` to configure this setting.
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserAssetDebtResponse {
    /// Asset denom
    pub denom: String,
    /// Scaled debt amount stored in contract state
    pub amount_scaled: Uint128,
    /// Underlying asset amount that is actually owed at the current block
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserAssetCollateralResponse {
    /// Asset denom
    pub denom: String,
    /// Scaled amount stored in contract state
    pub amount_scaled: Uint128,
    /// Underlying asset amount that is actually owed at the current block
    pub amount: Uint128,
    /// Whether this collateral is enabled
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserPositionResponse {
    pub total_collateral_in_base_asset: Uint128,
    pub total_debt_in_base_asset: Uint128,
    /// Total debt minus the uncollateralized debt
    pub total_collateralized_debt_in_base_asset: Uint128,
    pub max_debt_in_base_asset: Uint128,
    pub weighted_liquidation_threshold_in_base_asset: Uint128,
    pub health_status: UserHealthStatus,
}
