use cosmwasm_std::{CheckedMultiplyRatioError, DecimalRangeExceeded, OverflowError, StdError};
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
}

pub type ContractResult<T> = Result<T, ContractError>;
