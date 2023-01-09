use cosmwasm_std::{CheckedFromRatioError, CheckedMultiplyRatioError, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum HealthError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    CheckedMultiplyRatio(#[from] CheckedMultiplyRatioError),

    #[error("{0}")]
    CheckedFromRatio(#[from] CheckedFromRatioError),
}
