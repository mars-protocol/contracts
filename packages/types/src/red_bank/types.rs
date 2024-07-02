use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};
use cw_paginate::PaginationResponse;

use crate::red_bank::Market;

/// Global configuration
#[cw_serde]
pub struct Config<T> {
    /// Address provider returns addresses for all protocol contracts
    pub address_provider: T,
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

#[cw_serde]
pub struct ConfigResponse {
    /// The contract's owner
    pub owner: Option<String>,
    /// The contract's proposed owner
    pub proposed_new_owner: Option<String>,
    /// Address provider returns addresses for all protocol contracts
    pub address_provider: String,
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

pub type PaginatedUserCollateralResponse = PaginationResponse<UserCollateralResponse>;

#[cw_serde]
pub struct UserPositionResponse {
    /// Total value of all enabled collateral assets.
    /// If an asset is disabled as collateral, it will not be included in this value.
    pub total_enabled_collateral: Uint128,
    /// Total value of all collateralized debts.
    /// If the user has an uncollateralized loan limit in an asset, the debt in this asset will not
    /// be included in this value.
    pub total_collateralized_debt: Uint128,
    pub weighted_max_ltv_collateral: Uint128,
    pub weighted_liquidation_threshold_collateral: Uint128,
    pub health_status: UserHealthStatus,
}

#[cw_serde]
pub struct MarketV2Response {
    pub collateral_total_amount: Uint128,
    pub debt_total_amount: Uint128,
    pub utilization_rate: Decimal,

    #[serde(flatten)]
    pub market: Market,
}
