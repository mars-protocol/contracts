use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyRatioError, DecimalRangeExceeded, OverflowError,
    StdError, Uint128,
};
use thiserror::Error;

use crate::coins::Coins;

pub type ContractResult<T> = Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("Actions resulted in exceeding maximum allowed loan-to-value. Max LTV health factor: {max_ltv_health_factor:?}")]
    AboveMaxLTV {
        account_id: String,
        max_ltv_health_factor: String,
    },

    #[error("{0} is not an available coin to request")]
    CoinNotAvailable(String),

    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("{0}")]
    CheckedMultiply(#[from] CheckedMultiplyRatioError),

    #[error("{0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("Callbacks cannot be invoked externally")]
    ExternalInvocation,

    #[error("Extra funds received: {0}")]
    ExtraFundsReceived(Coins),

    #[error("Sent fund mismatch. Expected: {expected:?}, received {received:?}")]
    FundsMismatch {
        expected: Uint128,
        received: Uint128,
    },

    #[error(
        "Actions did not result in improved health factor: before: {prev_hf:?}, after: {new_hf:?}"
    )]
    HealthNotImproved { prev_hf: String, new_hf: String },

    #[error("No coin amount set for action")]
    NoAmount,

    #[error("No debt to repay")]
    NoDebt,

    #[error(
        "{account_id:?} is not a liquidatable credit account. Health factor: {lqdt_health_factor:?}."
    )]
    NotLiquidatable {
        account_id: String,
        lqdt_health_factor: String,
    },

    #[error("{user:?} is not the owner of {account_id:?}")]
    NotTokenOwner { user: String, account_id: String },

    #[error("{0} is not whitelisted")]
    NotWhitelisted(String),

    #[error("Expected vault coins in exchange for deposit, but none were sent")]
    NoVaultCoinsReceived,

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    RequirementsNotMet(String),

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{user:?} is not authorized to {action:?}")]
    Unauthorized { user: String, action: String },
}
