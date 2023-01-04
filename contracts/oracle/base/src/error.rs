use cosmwasm_std::StdError;
use mars_owner::OwnerError;
use thiserror::Error;

use mars_outpost::error::MarsError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("{0}")]
    OwnerError(#[from] OwnerError),

    #[error("Invalid price source: {reason}")]
    InvalidPriceSource {
        reason: String,
    },
}

pub type ContractResult<T> = Result<T, ContractError>;
