use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyRatioError, ConversionOverflowError, OverflowError,
    StdError,
};
use mars_owner::OwnerError;
use mars_red_bank_types::error::MarsError;
use mars_utils::error::ValidationError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("{0}")]
    Validation(#[from] ValidationError),

    #[error("{0}")]
    Version(#[from] cw2::VersionError),

    #[error("{0}")]
    Owner(#[from] OwnerError),

    #[error("{0}")]
    ConversionOverflow(#[from] ConversionOverflowError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    CheckedMultiplyRatio(#[from] CheckedMultiplyRatioError),

    #[error("{0}")]
    CheckedFromRatio(#[from] CheckedFromRatioError),

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
