use cosmwasm_std::{OverflowError, StdError};
use mars_core::error::MarsError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("Price source is not TWAP")]
    PriceSourceNotTwap {},

    #[error("Native price not found")]
    NativePriceNotFound {},

    #[error("No TWAP snapshot within tolerance")]
    NoSnapshotWithinTolerance {},

    #[error("Invalid pair")]
    InvalidPair {},
}
