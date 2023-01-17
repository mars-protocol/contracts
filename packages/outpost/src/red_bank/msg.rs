use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};
use mars_owner::OwnerUpdate;

use crate::red_bank::InterestRateModel;

#[cw_serde]
pub struct InstantiateMsg {
    /// Contract's owner
    pub owner: String,
    /// Contract's emergency owner
    pub emergency_owner: String,
    /// Market configuration
    pub config: CreateOrUpdateConfig,
}

#[cw_serde]
#[allow(clippy::large_enum_variant)]
pub enum ExecuteMsg {
    /// Manages owner state
    UpdateOwner(OwnerUpdate),

    /// Manages emergency owner state
    UpdateEmergencyOwner(OwnerUpdate),

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

    /// Update uncollateralized loan limit for a given user and asset.
    /// Overrides previous value if any. A limit of zero means no
    /// uncollateralized limit and the debt in that asset needs to be
    /// collateralized (only owner can call)
    UpdateUncollateralizedLoanLimit {
        /// Address that receives the credit
        user: String,
        /// Asset the user receives the credit in
        denom: String,
        /// Limit for the uncolateralize loan.
        new_limit: Uint128,
    },

    /// Deposit native coins. Deposited coins must be sent in the transaction
    /// this call is made
    Deposit {
        /// Address that will receive the maTokens
        on_behalf_of: Option<String>,
    },

    /// Withdraw an amount of the asset burning an equivalent amount of maTokens.
    Withdraw {
        /// Asset to withdraw
        denom: String,
        /// Amount to be withdrawn. If None is specified, the full amount will be withdrawn.
        amount: Option<Uint128>,
        /// The address where the withdrawn amount is sent
        recipient: Option<String>,
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
    pub close_factor: Option<Decimal>,
}

#[cw_serde]
pub struct InitOrUpdateAssetParams {
    /// Portion of the borrow rate that is kept as protocol rewards
    pub reserve_factor: Option<Decimal>,
    /// Max uusd that can be borrowed per uusd of collateral when using the asset as collateral
    pub max_loan_to_value: Option<Decimal>,
    /// uusd amount in debt position per uusd of asset collateral that if surpassed makes the user's position liquidatable.
    pub liquidation_threshold: Option<Decimal>,
    /// Bonus amount of collateral liquidator get when repaying user's debt (Will get collateral
    /// from user in an amount equal to debt repayed + bonus)
    pub liquidation_bonus: Option<Decimal>,

    /// Interest rate strategy to calculate borrow_rate and liquidity_rate
    pub interest_rate_model: Option<InterestRateModel>,

    /// If false cannot deposit
    pub deposit_enabled: Option<bool>,
    /// If false cannot borrow
    pub borrow_enabled: Option<bool>,
    /// Deposit Cap defined in terms of the asset (Unlimited by default)
    pub deposit_cap: Option<Uint128>,
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

    /// Enumerate markets with pagination
    #[returns(Vec<crate::red_bank::Market>)]
    Markets {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Get uncollateralized limit for given user and asset
    #[returns(crate::red_bank::UncollateralizedLoanLimitResponse)]
    UncollateralizedLoanLimit {
        user: String,
        denom: String,
    },

    /// Get all uncollateralized limits for a given user
    #[returns(Vec<crate::red_bank::UncollateralizedLoanLimitResponse>)]
    UncollateralizedLoanLimits {
        user: String,
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
        denom: String,
    },

    /// Get all collateral positions for a user
    #[returns(Vec<crate::red_bank::UserCollateralResponse>)]
    UserCollaterals {
        user: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Get user position
    #[returns(crate::red_bank::UserPositionResponse)]
    UserPosition {
        user: String,
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
