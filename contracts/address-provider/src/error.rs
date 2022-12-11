use cosmwasm_std::StdError;
use cw_controllers_admin_fork::AdminError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    AdminError(#[from] AdminError),

    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Invalid chain prefix: {0}")]
    InvalidChainPrefix(String),
}
