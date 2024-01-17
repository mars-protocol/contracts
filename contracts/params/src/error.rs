use cosmwasm_std::{DecimalRangeExceeded, StdError};
use cw2::VersionError;
use mars_owner::OwnerError;
use mars_types::error::MarsError;
use mars_utils::error::ValidationError;
use thiserror::Error;

pub type ContractResult<T> = Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("{0}")]
    Owner(#[from] OwnerError),

    #[error("{0}")]
    Validation(#[from] ValidationError),

    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("{0}")]
    Version(#[from] VersionError),
}
