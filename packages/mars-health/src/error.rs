use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

pub type MarsHealthResult<T> = Result<T, MarsHealthError>;

#[derive(Error, Debug, PartialEq)]
pub enum MarsHealthError {
    #[error("Invalid Debt.")]
    InvalidDebt {},
    #[error("{0}")]
    Overflow(#[from] OverflowError),
    #[error("{0}")]
    Std(#[from] StdError),
}