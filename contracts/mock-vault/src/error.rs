use cosmwasm_std::{CheckedMultiplyRatioError, StdError};
use rover::error::ContractError as RoverError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    RoverError(#[from] RoverError),

    #[error("{0}")]
    CheckedMultiply(#[from] CheckedMultiplyRatioError),
}
