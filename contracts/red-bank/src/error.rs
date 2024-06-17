use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyFractionError, DivideByZeroError, OverflowError, StdError,
};
use cw_utils::PaymentError;
use mars_health::error::HealthError;
use mars_liquidation::error::LiquidationError;
use mars_owner::OwnerError;
use mars_types::error::MarsError;
use mars_utils::error::{GuardError, ValidationError};
use thiserror::Error;

pub type ContractResult<T> = Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("{0}")]
    Validation(#[from] ValidationError),

    #[error("{0}")]
    Owner(#[from] OwnerError),

    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    CheckedFromRatio(#[from] CheckedFromRatioError),

    #[error("{0}")]
    CheckedMultiplyFraction(#[from] CheckedMultiplyFractionError),

    #[error("{0}")]
    DivideByZero(#[from] DivideByZeroError),

    #[error("{0}")]
    Health(#[from] HealthError),

    #[error("{0}")]
    Liquidation(#[from] LiquidationError),

    #[error("Price not found for asset: {denom:?}")]
    PriceNotFound {
        denom: String,
    },

    #[error("User address {user:?} has no balance in specified collateral asset {denom:?}")]
    UserNoCollateralBalance {
        user: String,
        denom: String,
    },

    #[error(
        "Withdraw amount must be greater than 0 and less or equal user balance (asset: {denom:?})"
    )]
    InvalidWithdrawAmount {
        denom: String,
    },

    #[error("User's health factor can't be less than 1 after withdraw")]
    InvalidHealthFactorAfterWithdraw {},

    #[error("Asset is already initialized")]
    AssetAlreadyInitialized {},

    #[error("Asset not initialized")]
    AssetNotInitialized {},

    #[error("Deposit Cap exceeded for {denom:?}")]
    DepositCapExceeded {
        denom: String,
    },

    #[error("Cannot have 0 as liquidity index")]
    InvalidLiquidityIndex {},

    #[error("Borrow amount must be greater than 0 and less or equal available liquidity (asset: {denom:?})")]
    InvalidBorrowAmount {
        denom: String,
    },

    #[error("Borrow amount exceeds maximum allowed given current collateral value")]
    BorrowAmountExceedsGivenCollateral {},

    #[error("Cannot repay 0 debt")]
    CannotRepayZeroDebt {},

    #[error("Amount to repay is greater than total debt")]
    CannotRepayMoreThanDebt {},

    #[error("User cannot issue liquidation of own account")]
    CannotLiquidateSelf {},

    #[error("User can't be liquidated for asset {denom:?} not being used as collateral")]
    CannotLiquidateWhenCollateralUnset {
        denom: String,
    },

    #[error("User has no balance in specified collateral asset to be liquidated")]
    CannotLiquidateWhenNoCollateralBalance {},

    #[error(
        "User has no outstanding debt in the specified debt asset and thus cannot be liquidated"
    )]
    CannotLiquidateWhenNoDebtBalance {},

    #[error("User's health factor is not less than 1 and thus cannot be liquidated")]
    CannotLiquidateHealthyPosition {},

    #[error("Contract does not have enough collateral liquidity to send back underlying asset")]
    CannotLiquidateWhenNotEnoughCollateral {},

    #[error(
        "Cannot make token transfer if it results in a health factor lower than 1 for the sender"
    )]
    CannotTransferTokenWhenInvalidHealthFactor {},

    #[error("Failed to encode asset reference into string")]
    CannotEncodeAssetReferenceIntoString {},

    #[error("Deposit for {denom:?} is not enabled")]
    DepositNotEnabled {
        denom: String,
    },

    #[error("Borrow for {denom:?} is not enabled")]
    BorrowNotEnabled {
        denom: String,
    },

    #[error("Cannot liquidate. Debt asset {denom:?}")]
    LiquidationNotAllowedWhenDebtMarketInactive {
        denom: String,
    },

    #[error("User's health factor can't be less than 1 after disabling collateral")]
    InvalidHealthFactorAfterDisablingCollateral {},

    #[error("{0}")]
    Version(#[from] cw2::VersionError),

    #[error("{0}")]
    Guard(#[from] GuardError),

    #[error("Cannot repay on behalf of credit manager")]
    CannotRepayOnBehalfOfCreditManager {},

    #[error("Cannot liquidate credit manager (use credit-manager contract liquidate function)")]
    CannotLiquidateCreditManager {},
}
