use cosmwasm_std::{CheckedMultiplyRatioError, DecimalRangeExceeded, OverflowError, StdError};
use rover::error::ContractError as RoverError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("Invalid route: {reason}")]
    InvalidRoute { reason: String },

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    CheckedMultiplyRatio(#[from] CheckedMultiplyRatioError),

    #[error("{denom_a:?}-{denom_b:?} is not an available pool")]
    PoolNotFound { denom_a: String, denom_b: String },

    #[error("{0}")]
    Rover(#[from] RoverError),

    #[error("{0}")]
    Std(#[from] StdError),
}

pub type ContractResult<T> = Result<T, ContractError>;
