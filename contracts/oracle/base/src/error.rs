use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyFractionError, CheckedMultiplyRatioError,
    ConversionOverflowError, DecimalRangeExceeded, DivideByZeroError, OverflowError, StdError,
};
use mars_owner::OwnerError;
use mars_utils::error::ValidationError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

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
    CheckedMultiplyFraction(#[from] CheckedMultiplyFractionError),

    #[error("{0}")]
    CheckedFromRatio(#[from] CheckedFromRatioError),

    #[error("{0}")]
    DivideByZero(#[from] DivideByZeroError),

    #[error("{0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("Invalid price source: {reason}")]
    InvalidPriceSource {
        reason: String,
    },

    #[error("Invalid price: {reason}")]
    InvalidPrice {
        reason: String,
    },

    #[error("Missing custom init params")]
    MissingCustomInitParams {},

    #[error("Missing custom execute params")]
    MissingCustomExecuteParams {},

    #[error("Price source is not TWAP")]
    PriceSourceNotTwap {},

    #[error("No TWAP snapshot within tolerance")]
    NoSnapshotWithinTolerance {},

    #[error("No TWAP snapshots found")]
    NoSnapshots {},

    #[error("There needs to be at least two TWAP snapshots")]
    NotEnoughSnapshots {},

    #[error("Invalid pair type")]
    InvalidPairType {},

    #[error("Snapshots have the same cumulative price. This should never happen.")]
    InvalidCumulativePrice {},

    #[error("Missing astroport pool params")]
    MissingAstroportPoolParams {},
}

pub type ContractResult<T> = Result<T, ContractError>;
