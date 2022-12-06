use cosmwasm_std::{CheckedFromRatioError, DecimalRangeExceeded, OverflowError, StdError};
use cw_controllers_admin_fork::AdminError;
use thiserror::Error;

use mars_rover::error::ContractError as RoverError;

pub type ContractResult<T> = Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    AdminError(#[from] AdminError),

    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("{0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    RoverError(#[from] RoverError),

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{user:?} is not authorized to {action:?}")]
    Unauthorized { user: String, action: String },
}
