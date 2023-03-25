use cosmwasm_std::{ConversionOverflowError, OverflowError, StdError};
// use mars_oracle::error::MarsError;
use mars_owner::OwnerError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    // #[error("{0}")]
    // Mars(#[from] MarsError),
    #[error("Invalid denom: {reason}")]
    InvalidDenom {
        reason: String,
    },

    #[error("{0}")]
    Version(#[from] cw2::VersionError),

    #[error("{0}")]
    Owner(#[from] OwnerError),

    #[error("{0}")]
    ConversionOverflow(#[from] ConversionOverflowError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

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
