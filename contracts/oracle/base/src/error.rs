use cosmwasm_std::{ConversionOverflowError, StdError};
use mars_outpost::error::MarsError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("{0}")]
    ConversionOverflowError(#[from] ConversionOverflowError),

    #[error("Invalid price source: {reason}")]
    InvalidPriceSource {
        reason: String,
    },

    #[error("Invalid price: {reason}")]
    InvalidPrice {
        reason: String,
    },
}

pub type ContractResult<T> = Result<T, ContractError>;
