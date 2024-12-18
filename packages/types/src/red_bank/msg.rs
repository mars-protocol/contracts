use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};
use mars_owner::OwnerUpdate;

use crate::red_bank::InterestRateModel;

#[cw_serde]
pub struct InstantiateMsg {
    /// Contract's owner
    pub owner: String,
    /// Market configuration
    pub config: CreateOrUpdateConfig,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Manages owner state
    UpdateOwner(OwnerUpdate),

    /// Update contract config (only owner can call)
    UpdateConfig {
        config: CreateOrUpdateConfig,
    },

    /// Initialize an asset on the money market (only owner can call)
    InitAsset {
        /// Asset related info
        denom: String,
        /// Asset parameters
        params: InitOrUpdateAssetParams,
    },

    /// Update an asset on the money market (only owner can call)
    UpdateAsset {
        /// Asset related info
        denom: String,
        /// Asset parameters
        params: InitOrUpdateAssetParams,
    },

    /// Deposit native coins. Deposited coins must be sent in the transaction
    /// this call is made
    Deposit {
        /// Credit account id (Rover)
        account_id: Option<String>,

        /// Address that will receive the coins
        on_behalf_of: Option<String>,
    },

    /// Withdraw native coins
    Withdraw {
        /// Asset to withdraw
        denom: String,
        /// Amount to be withdrawn. If None is specified, the full amount will be withdrawn.
        amount: Option<Uint128>,
        /// The address where the withdrawn amount is sent
        recipient: Option<String>,
        /// Credit account id (Rover)
        account_id: Option<String>,
        // Withdraw action related to liquidation process initiated in credit manager.
        // This flag is used to identify different way for pricing assets during liquidation.
        liquidation_related: Option<bool>,
    },

    /// Borrow native coins. If borrow allowed, amount is added to caller's debt
    /// and sent to the address.
    Borrow {
        /// Asset to borrow
        denom: String,
        /// Amount to borrow
        amount: Uint128,
        /// The address where the borrowed amount is sent
        recipient: Option<String>,
    },

    /// Repay native coins loan. Coins used to repay must be sent in the
    /// transaction this call is made.
    Repay {
        /// Repay the funds for the user
        on_behalf_of: Option<String>,
    },

    /// Liquidate under-collateralized native loans. Coins used to repay must be sent in the
    /// transaction this call is made.
    ///
    /// The liquidator will receive collateral shares. To get the underlying asset, consider sending
    /// a separate `withdraw` execute message.
    Liquidate {
        /// The address of the borrower getting liquidated
        user: String,
        /// Denom of the collateral asset, which liquidator gets from the borrower
        collateral_denom: String,
        /// The address for receiving underlying collateral
        recipient: Option<String>,
    },

    /// Update (enable / disable) asset as collateral for the caller
    UpdateAssetCollateralStatus {
        /// Asset to update status for
        denom: String,
        /// Option to enable (true) / disable (false) asset as collateral
        enable: bool,
    },
}

#[cw_serde]
pub struct CreateOrUpdateConfig {
    pub address_provider: Option<String>,
}

#[cw_serde]
pub struct InitOrUpdateAssetParams {
    /// Portion of the borrow rate that is kept as protocol rewards
    pub reserve_factor: Option<Decimal>,

    /// Interest rate strategy to calculate borrow_rate and liquidity_rate
    pub interest_rate_model: Option<InterestRateModel>,
}

/// Migrate from V1 to V2, only owner can call
#[cw_serde]
pub enum MigrateV1ToV2 {
    /// Migrate collaterals in batches
    Collaterals {
        limit: u32,
    },
    /// Clears old V1 state once all batches are migrated or after a certain time
    ClearV1State {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get config
    #[returns(crate::red_bank::ConfigResponse)]
    Config {},

    /// Get asset market
    #[returns(crate::red_bank::Market)]
    Market {
        denom: String,
    },

    /// Get asset market with underlying collateral and debt amount
    #[returns(crate::red_bank::MarketV2Response)]
    MarketV2 {
        denom: String,
    },

    /// Enumerate markets with pagination
    #[returns(Vec<crate::red_bank::Market>)]
    Markets {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Enumerate marketsV2 with pagination
    #[returns(cw_paginate::PaginationResponse<crate::red_bank::MarketV2Response>)]
    MarketsV2 {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Get user debt position for a specific asset
    #[returns(crate::red_bank::UserDebtResponse)]
    UserDebt {
        user: String,
        denom: String,
    },

    /// Get all debt positions for a user
    #[returns(Vec<crate::red_bank::UserDebtResponse>)]
    UserDebts {
        user: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Get user collateral position for a specific asset
    #[returns(crate::red_bank::UserCollateralResponse)]
    UserCollateral {
        user: String,
        account_id: Option<String>,
        denom: String,
    },

    /// Get all collateral positions for a user
    #[returns(Vec<crate::red_bank::UserCollateralResponse>)]
    UserCollaterals {
        user: String,
        account_id: Option<String>,
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Get all collateral positions for a user
    #[returns(crate::red_bank::PaginatedUserCollateralResponse)]
    UserCollateralsV2 {
        user: String,
        account_id: Option<String>,
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Get user position
    #[returns(crate::red_bank::UserPositionResponse)]
    UserPosition {
        user: String,
        account_id: Option<String>,
    },

    /// Get user position for liquidation
    #[returns(crate::red_bank::UserPositionResponse)]
    UserPositionLiquidationPricing {
        user: String,
        account_id: Option<String>,
    },

    /// Get liquidity scaled amount for a given underlying asset amount.
    /// (i.e: how much scaled collateral is added if the given amount is deposited)
    #[returns(Uint128)]
    ScaledLiquidityAmount {
        denom: String,
        amount: Uint128,
    },

    /// Get equivalent scaled debt for a given underlying asset amount.
    /// (i.e: how much scaled debt is added if the given amount is borrowed)
    #[returns(Uint128)]
    ScaledDebtAmount {
        denom: String,
        amount: Uint128,
    },

    /// Get underlying asset amount for a given asset and scaled amount.
    /// (i.e. How much underlying asset will be released if withdrawing by burning a given scaled
    /// collateral amount stored in state.)
    #[returns(Uint128)]
    UnderlyingLiquidityAmount {
        denom: String,
        amount_scaled: Uint128,
    },

    /// Get underlying debt amount for a given asset and scaled amounts.
    /// (i.e: How much underlying asset needs to be repaid to cancel a given scaled debt
    /// amount stored in state)
    #[returns(Uint128)]
    UnderlyingDebtAmount {
        denom: String,
        amount_scaled: Uint128,
    },
}
