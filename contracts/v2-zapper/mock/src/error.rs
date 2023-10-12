use cosmwasm_std::{CheckedMultiplyRatioError, StdError};
use cw_utils::PaymentError;
use mars_types::adapters::oracle::OracleError;
use thiserror::Error;

pub type ContractResult<T> = Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    OracleError(#[from] OracleError),

    #[error("{0}")]
    CheckedMultiply(#[from] CheckedMultiplyRatioError),

    #[error("Required minimum received was not met")]
    ReceivedBelowMinimum,

    #[error("Could not find coin trying to access")]
    CoinNotFound,

    #[error("{0}")]
    RequirementsNotMet(String),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),
}
