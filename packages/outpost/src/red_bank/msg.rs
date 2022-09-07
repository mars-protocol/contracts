use cosmwasm_std::{Addr, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::red_bank::InterestRateModel;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    /// Market configuration
    pub config: CreateOrUpdateConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
pub enum ExecuteMsg {
    /// Update contract config (only owner can call)
    UpdateConfig {
        config: CreateOrUpdateConfig,
    },

    /// Initialize an asset on the money market (only owner can call)
    InitAsset {
        /// Asset related info
        denom: String,
        /// Asset parameters
        asset_params: InitOrUpdateAssetParams,
        /// Asset symbol to be used in maToken name and description. If non is provided,
        /// denom will be used for native and token symbol will be used for cw20. Mostly
        /// useful for native assets since it's denom (e.g.: uluna, uusd) does not match it's
        /// user facing symbol (LUNA, UST) which should be used in maToken's attributes
        /// for the sake of consistency
        asset_symbol: Option<String>,
    },

    /// Callback sent from maToken contract after instantiated
    InitAssetTokenCallback {
        denom: String,
    },

    /// Update an asset on the money market (only owner can call)
    UpdateAsset {
        /// Asset related info
        denom: String,
        /// Asset parameters
        asset_params: InitOrUpdateAssetParams,
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
        /// Amount to be withdrawn. If None is specified, the full maToken balance will be
        /// burned in exchange for the equivalent asset amount.
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
    },

    /// Update (enable / disable) asset as collateral for the caller
    UpdateAssetCollateralStatus {
        /// Asset to update status for
        denom: String,
        /// Option to enable (true) / disable (false) asset as collateral
        enable: bool,
    },

    /// Called by liquidity token (maToken). Validate liquidity token transfer is valid
    /// and update collateral status
    FinalizeLiquidityTokenTransfer {
        /// Token sender. Address is trusted because it should have been verified in
        /// the token contract
        sender_address: Addr,
        /// Token recipient. Address is trusted because it should have been verified in
        /// the token contract
        recipient_address: Addr,
        /// Sender's balance before the token transfer
        sender_previous_balance: Uint128,
        /// Recipient's balance before the token transfer
        recipient_previous_balance: Uint128,
        /// Transfer amount
        amount: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CreateOrUpdateConfig {
    pub owner: Option<String>,
    pub address_provider: Option<String>,
    pub ma_token_code_id: Option<u64>,
    pub close_factor: Option<Decimal>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InitOrUpdateAssetParams {
    /// Initial borrow rate
    pub initial_borrow_rate: Option<Decimal>,

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get config
    Config {},

    /// Get asset market
    Market {
        denom: String,
    },

    /// Enumerate markets with pagination. Returns Vec<Market>
    Markets {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Get uncollateralized limit for given user and asset.
    /// Returns UncollateralizedLoanLimitResponse
    UncollateralizedLoanLimit {
        user: String,
        denom: String,
    },

    /// Get all uncollateralized limits for a given user.
    /// Returns Vec<UncollateralizedLoanLimitResponse>
    UncollateralizedLoanLimits {
        user: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Get user debt position for a specific asset. Returns UserDebtResponse
    UserDebt {
        user: String,
        denom: String,
    },

    /// Get all debt positions for a user. Returns Vec<UserDebtResponse>
    UserDebts {
        user: String,
        // TODO: Given the current approach Red Bank uses to track user positions (bitarrays),
        // pagination is not possible. This will be addressed in an upcoming PR.
        // start_after: Option<String>,
        // limit: Option<u32>,
    },

    /// Get user collateral position for a specific asset. Returns UserCollateralResponse
    UserCollateral {
        user: String,
        denom: String,
    },

    /// Get all collateral positions for a user. Returns Vec<UserCollateralResponse>
    UserCollaterals {
        user: String,
        // TODO: Given the current approach Red Bank uses to track user positions (bitarrays),
        // pagination is not possible. This will be addressed in an upcoming PR.
        // start_after: Option<String>,
        // limit: Option<u32>,
    },

    /// Get user position. Returns UserPositionResponse
    UserPosition {
        user: String,
    },

    /// Get liquidity scaled amount for a given underlying asset amount
    /// (i.e: how much maTokens will get minted if the given amount is deposited)
    ScaledLiquidityAmount {
        denom: String,
        amount: Uint128,
    },

    /// Get equivalent scaled debt for a given underlying asset amount.
    /// (i.e: how much scaled debt is added if the given amount is borrowed)
    ScaledDebtAmount {
        denom: String,
        amount: Uint128,
    },

    /// Get underlying asset amount for a given maToken balance.
    UnderlyingLiquidityAmount {
        ma_token_address: String,
        amount_scaled: Uint128,
    },

    /// Get underlying debt amount for a given asset and scaled amounts.
    /// (i.e: How much underlying asset needs to be repaid to cancel a given scaled debt
    /// amount stored in state)
    UnderlyingDebtAmount {
        denom: String,
        amount_scaled: Uint128,
    },
}
