use cosmwasm_std::{OverflowError, StdError, DecimalRangeExceeded};
use thiserror::Error;

pub type MarsHealthResult<T> = Result<T, MarsHealthError>;

#[derive(Error, Debug, PartialEq)]
pub enum MarsHealthError {
    #[error("Total debt is zero. Cannot compute the health factor")]
    TotalDebtIsZero,

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("{0}")]
    Std(#[from] StdError),
}