use cosmwasm_std::{CheckedMultiplyRatioError, StdError};
use rover::error::ContractError as RoverError;
use thiserror::Error;

pub type ContractResult<T> = Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    RoverError(#[from] RoverError),

    #[error("{0}")]
    CheckedMultiply(#[from] CheckedMultiplyRatioError),

    #[error("Required minimum received was not met")]
    ReceivedBelowMinimum,

    #[error("Could not find coin trying to access")]
    CoinNotFound,

    #[error("{lp_token:?} requires {coin0:?} and {coin1:?}")]
    RequirementsNotMet {
        lp_token: String,
        coin0: String,
        coin1: String,
    },
}
