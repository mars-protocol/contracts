use cosmwasm_std::StdError;
use mars_owner::OwnerError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Owner(#[from] OwnerError),

    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Invalid chain prefix: {0}")]
    InvalidChainPrefix(String),

    #[error("{0}")]
    Version(#[from] cw2::VersionError),
}
