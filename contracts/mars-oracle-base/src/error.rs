use cosmwasm_std::StdError;
use thiserror::Error;

use mars_outpost::error::MarsError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("Invalid pool id: {reason}")]
    InvalidPoolId {
        reason: String,
    },
}

pub type ContractResult<T> = Result<T, ContractError>;
