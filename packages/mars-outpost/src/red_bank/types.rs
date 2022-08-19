use cosmwasm_std::{Addr, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::error::MarsError;
use crate::helpers::decimal_param_le_one;
use crate::red_bank;

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

/// Asset markets
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Market {
    /// Market index (Bit position on data)
    pub index: u32,
    /// Denom of the asset
    pub denom: String,
    /// maToken contract address
    pub ma_token_address: Addr,

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
            index: 0,
            denom: "".to_string(),
            ma_token_address: crate::helpers::zero_address(),
            liquidity_index: Decimal::one(),
            borrow_index: Decimal::one(),
            borrow_rate: Default::default(),
            liquidity_rate: Default::default(),
            max_loan_to_value: Default::default(),
            reserve_factor: Default::default(),
            indexes_last_updated: 0,
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
    Borrowing{
        max_ltv_hf: Decimal,
        liq_threshold_hf: Decimal,
    },
}

/// User asset settlement
#[derive(Default, Debug)]
pub struct UserAssetPosition {
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
    pub total_collateral_value: Decimal,
    pub total_debt_value: Decimal,
    /// Total debt minus the uncollateralized debt
    pub total_collateralized_debt: Decimal,
    pub weighted_max_ltv_collateral: Decimal,
    pub weighted_liquidation_threshold_collateral: Decimal,
    pub health_status: UserHealthStatus,
}
