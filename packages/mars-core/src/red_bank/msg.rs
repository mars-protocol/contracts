use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};

use cw20::Cw20ReceiveMsg;

use crate::asset::Asset;
use crate::math::decimal::Decimal;

use super::interest_rate_models::InterestRateModelParams;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub config: CreateOrUpdateConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Implementation of cw20 receive msg
    Receive(Cw20ReceiveMsg),

    /// Update contract config (only owner can call)
    UpdateConfig { config: CreateOrUpdateConfig },

    /// Initialize an asset on the money market (only owner can call)
    InitAsset {
        /// Asset related info
        asset: Asset,
        /// Asset parameters
        asset_params: InitOrUpdateAssetParams,
    },

    /// Callback sent from maToken contract after instantiated
    InitAssetTokenCallback {
        /// Either the denom for a terra native asset or address for a cw20 token
        /// in bytes
        reference: Vec<u8>,
    },

    /// Update an asset on the money market (only owner can call)
    UpdateAsset {
        /// Asset related info
        asset: Asset,
        /// Asset parameters
        asset_params: InitOrUpdateAssetParams,
    },

    /// Update uncollateralized loan limit for a given user and asset.
    /// Overrides previous value if any. A limit of zero means no
    /// uncollateralized limit and the debt in that asset needs to be
    /// collateralized (only owner can call)
    UpdateUncollateralizedLoanLimit {
        /// Address that receives the credit
        user_address: String,
        /// Asset the user receives the credit in
        asset: Asset,
        /// Limit for the uncolateralize loan.
        new_limit: Uint128,
    },

    /// Deposit Terra native coins. Deposited coins must be sent in the transaction
    /// this call is made
    DepositNative {
        /// Denom used in Terra (e.g: uluna, uusd)
        denom: String,
    },

    /// Withdraw an amount of the asset burning an equivalent amount of maTokens.
    /// If asset is a Terra native token, the amount sent to the user
    /// is selected so that the sum of the transfered amount plus the stability tax
    /// payed is equal to the withdrawn amount.
    Withdraw {
        /// Asset to withdraw
        asset: Asset,
        /// Amount to be withdrawn. If None is specified, the full maToken balance will be
        /// burned in exchange for the equivalent asset amount.
        amount: Option<Uint128>,
    },

    /// Borrow Terra native coins. If borrow allowed, amount is added to caller's debt
    /// and sent to the address. If asset is a Terra native token, the amount sent
    /// is selected so that the sum of the transfered amount plus the stability tax
    /// payed is equal to the borrowed amount.
    Borrow {
        /// Asset to borrow
        asset: Asset,
        /// Amount to borrow
        amount: Uint128,
    },

    /// Repay Terra native coins loan. Coins used to repay must be sent in the
    /// transaction this call is made.
    RepayNative {
        /// Denom used in Terra (e.g: uluna, uusd)
        denom: String,
    },

    /// Liquidate under-collateralized native loans. Coins used to repay must be sent in the
    /// transaction this call is made.
    LiquidateNative {
        /// Collateral asset liquidator gets from the borrower
        collateral_asset: Asset,
        /// Denom used in Terra (e.g: uluna, uusd) of the debt asset
        debt_asset_denom: String,
        /// The address of the borrower getting liquidated
        user_address: String,
        /// Whether the liquidator gets liquidated collateral in maToken (true) or
        /// the underlying collateral asset (false)
        receive_ma_token: bool,
    },

    /// Update (enable / disable) asset as collateral for the caller
    UpdateAssetCollateralStatus {
        /// Asset to update status for
        asset: Asset,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    /// Deposit sent cw20 tokens
    DepositCw20 {},
    /// Repay sent cw20 tokens
    RepayCw20 {},
    /// Liquidate under-collateralized cw20 loan using the sent cw20 tokens.
    LiquidateCw20 {
        /// Collateral asset liquidator gets from the borrower
        collateral_asset: Asset,
        /// The address of the borrower getting liquidated
        user_address: String,
        /// Whether the liquidator gets liquidated collateral in maToken (true) or
        /// the underlying collateral asset (false)
        receive_ma_token: bool,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CreateOrUpdateConfig {
    pub owner: Option<String>,
    pub address_provider_address: Option<String>,
    pub ma_token_code_id: Option<u64>,
    pub close_factor: Option<Decimal>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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
    pub interest_rate_model_params: Option<InterestRateModelParams>,

    /// If false cannot do any action (deposit/withdraw/borrow/repay/liquidate)
    pub active: Option<bool>,
    /// If false cannot deposit
    pub deposit_enabled: Option<bool>,
    /// If false cannot borrow
    pub borrow_enabled: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get config
    Config {},

    /// Get asset market
    Market { asset: Asset },

    /// Get a list of all markets. Returns MarketsListResponse
    MarketsList {},

    /// Get uncollateralized limit for given asset and user.
    /// Returns UncollateralizedLoanLimitResponse
    UncollateralizedLoanLimit { user_address: String, asset: Asset },

    /// Get all debt positions for a user. Returns UsetDebtResponse
    UserDebt { user_address: String },

    /// Get user debt position for a specific asset. Returns UserAssetDebtResponse
    UserAssetDebt { user_address: String, asset: Asset },

    /// Get info about whether or not user is using each asset as collateral.
    /// Returns UserCollateralResponse
    UserCollateral { user_address: String },

    /// Get user position. Returns UserPositionResponse
    UserPosition { user_address: String },

    /// Get liquidity scaled amount for a given underlying asset amount
    /// (i.e: how much maTokens will get minted if the given amount is deposited)
    ScaledLiquidityAmount { asset: Asset, amount: Uint128 },

    /// Get equivalent scaled debt for a given underlying asset amount.
    /// (i.e: how much scaled debt is added if the given amount is borrowed)
    ScaledDebtAmount { asset: Asset, amount: Uint128 },

    /// Get underlying asset amount for a given maToken balance.
    UnderlyingLiquidityAmount {
        ma_token_address: String,
        amount_scaled: Uint128,
    },

    /// Get underlying debt amount for a given asset and scaled amounts.
    /// (i.e: How much underlying asset needs to be repaid to cancel a given scaled debt
    /// amount stored in state)
    UnderlyingDebtAmount {
        asset: Asset,
        amount_scaled: Uint128,
    },
}
