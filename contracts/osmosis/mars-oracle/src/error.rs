use cosmwasm_std::{OverflowError, StdError};
use mars_outpost::error::MarsError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("Can't encode asset reference to utf8 string")]
    CannotEncodeToUtf8String,

    #[error("Invalid pool id")]
    InvalidPoolId {},
}

impl From<ContractError> for StdError {
    fn from(source: ContractError) -> Self {
        match source {
            ContractError::Std(e) => e,
            e => StdError::generic_err(format!("{}", e)),
        }
    }
}
