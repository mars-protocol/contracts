use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyFractionError, CheckedMultiplyRatioError, Coin,
    DecimalRangeExceeded, OverflowError, StdError, Uint128,
};
use mars_owner::OwnerError;
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

    #[error("Vault deposit would result in exceeding limit. With deposit: {new_value:?}, Maximum: {maximum:?}")]
    AboveVaultDepositCap {
        new_value: String,
        maximum: String,
    },

    #[error("{0}")]
    Owner(#[from] OwnerError),

    #[error("{0} is not an available coin to request")]
    CoinNotAvailable(String),

    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("{0}")]
    CheckedMultiply(#[from] CheckedMultiplyRatioError),

    #[error("{0}")]
    CheckedMultiplyFraction(#[from] CheckedMultiplyFractionError),

    #[error("{0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("New unlocking positions: {new_amount:?}. Maximum: {maximum:?}.")]
    ExceedsMaxUnlockingPositions {
        new_amount: Uint128,
        maximum: Uint128,
    },

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
    HealthNotImproved {
        prev_hf: String,
        new_hf: String,
    },

    #[error("{reason:?}")]
    InvalidConfig {
        reason: String,
    },

    #[error("Paying down {debt_coin:?} for {request_coin:?} does not result in a profit for the liquidator")]
    LiquidationNotProfitable {
        debt_coin: Coin,
        request_coin: Coin,
    },

    #[error("Issued incorrect action for vault type")]
    MismatchedVaultType,

    #[error("No coin amount set for action")]
    NoAmount,

    #[error("No debt to repay")]
    NoDebt,

    #[error("Nothing lent to reclaim")]
    NoneLent,

    #[error("Position {0} was not a valid position for this account id in this vault")]
    NoPositionMatch(String),

    #[error(
        "{account_id:?} is not a liquidatable credit account. Health factor: {lqdt_health_factor:?}."
    )]
    NotLiquidatable {
        account_id: String,
        lqdt_health_factor: String,
    },

    #[error("{user:?} is not the owner of {account_id:?}")]
    NotTokenOwner {
        user: String,
        account_id: String,
    },

    #[error("{0} is not whitelisted")]
    NotWhitelisted(String),

    #[error("Expected vault coins in exchange for deposit, but none were sent")]
    NoVaultCoinsReceived,

    #[error("No more than one vault positions is allowed")]
    OnlyOneVaultPositionAllowed,

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("Reply id: {0} not valid")]
    ReplyIdError(u64),

    #[error("{0}")]
    RequirementsNotMet(String),

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{user:?} is not authorized to {action:?}")]
    Unauthorized {
        user: String,
        action: String,
    },

    #[error("There is more time left on the lock period")]
    UnlockNotReady,
}
