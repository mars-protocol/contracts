use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};

use crate::error::MarsError;
use crate::helpers::decimal_param_le_one;

/// Global configuration
#[cw_serde]
pub struct Config<T> {
    /// Contract owner
    pub owner: T,
    /// Address provider returns addresses for all protocol contracts
    pub address_provider: T,
    /// Maximum percentage of outstanding debt that can be covered by a liquidator
    pub close_factor: Decimal,
}

impl<T> Config<T> {
    pub fn validate(&self) -> Result<(), MarsError> {
        decimal_param_le_one(self.close_factor, "close_factor")?;
        Ok(())
    }
}

#[cw_serde]
#[derive(Default)]
pub struct Collateral {
    /// Scaled collateral amount
    pub amount_scaled: Uint128,
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
#[cw_serde]
#[derive(Default)]
pub struct Debt {
    /// Scaled debt amount
    pub amount_scaled: Uint128,
    /// Marker for uncollateralized debt
    pub uncollateralized: bool,
}

#[cw_serde]
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

pub type ConfigResponse = Config<String>;

#[cw_serde]
pub struct UncollateralizedLoanLimitResponse {
    /// Asset denom
    pub denom: String,
    /// Uncollateralized loan limit in this asset
    pub limit: Uint128,
}

#[cw_serde]
pub struct UserDebtResponse {
    /// Asset denom
    pub denom: String,
    /// Scaled debt amount stored in contract state
    pub amount_scaled: Uint128,
    /// Underlying asset amount that is actually owed at the current block
    pub amount: Uint128,
    /// Marker for uncollateralized debt
    pub uncollateralized: bool,
}

#[cw_serde]
pub struct UserCollateralResponse {
    /// Asset denom
    pub denom: String,
    /// Scaled collateral amount stored in contract state
    pub amount_scaled: Uint128,
    /// Underlying asset amount that is actually deposited at the current block
    pub amount: Uint128,
    /// Wether the user is using asset as collateral or not
    pub enabled: bool,
}

#[cw_serde]
pub struct UserPositionResponse {
    /// Total value of all enabled collateral assets.
    /// If an asset is disabled as collateral, it will not be included in this value.
    pub total_enabled_collateral: Decimal,
    /// Total value of all collateralized debts.
    /// If the user has an uncollateralized loan limit in an asset, the debt in this asset will not
    /// be included in this value.
    pub total_collateralized_debt: Decimal,
    pub weighted_max_ltv_collateral: Decimal,
    pub weighted_liquidation_threshold_collateral: Decimal,
    pub health_status: UserHealthStatus,
}
