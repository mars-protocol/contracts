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

    #[error("This vault does not require a lockup, just withdraw directly")]
    NoLockupTime,

    #[error("Lockup position {0} not found")]
    LockupPositionNotFound(u64),

    #[error("There is more time left on the lock period")]
    UnlockNotReady,

    #[error("You must request an unlock first")]
    UnlockRequired,

    #[error("Vault token not sent")]
    VaultTokenNotSent,
}
