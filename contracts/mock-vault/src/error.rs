use cosmwasm_std::{CheckedMultiplyRatioError, StdError};
use mars_types::adapters::oracle::OracleError;
use thiserror::Error;

pub type ContractResult<T> = Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Oracle(#[from] OracleError),

    #[error("{0}")]
    CheckedMultiply(#[from] CheckedMultiplyRatioError),

    #[error("Lockup position {0} not found")]
    LockupPositionNotFound(u64),

    #[error("Attempting to deposit, but did not sent any tokens")]
    NoCoinsSent,

    #[error("This vault is not a locking vault, action not allowed")]
    NotLockingVault,

    #[error("Not allowed to perform action")]
    Unauthorized,

    #[error("There is more time left on the lock period")]
    UnlockNotReady,

    #[error("You must request an unlock first")]
    UnlockRequired,

    #[error("Attempting to deposit incorrect denom")]
    WrongDenomSent,

    #[error("Vault token not sent")]
    VaultTokenNotSent,
}
