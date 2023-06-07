use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyFractionError, CheckedMultiplyRatioError,
    DecimalRangeExceeded, OverflowError, StdError,
};
use mars_owner::OwnerError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    OwnerError(#[from] OwnerError),

    #[error("{0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("Invalid route: {reason}")]
    InvalidRoute {
        reason: String,
    },

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    CheckedMultiplyRatio(#[from] CheckedMultiplyRatioError),

    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("{0}")]
    CheckedMultiplyFractionError(#[from] CheckedMultiplyFractionError),

    #[error("{denom_a:?}-{denom_b:?} is not an available pool")]
    PoolNotFound {
        denom_a: String,
        denom_b: String,
    },

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{user:?} is not authorized to {action:?}")]
    Unauthorized {
        user: String,
        action: String,
    },

    #[error("No route found from {from} to {to}")]
    NoRoute {
        from: String,
        to: String,
    },
}

pub type ContractResult<T> = Result<T, ContractError>;
