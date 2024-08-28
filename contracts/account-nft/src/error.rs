use cosmwasm_std::{OverflowError, StdError};
use cw721_base::ContractError as Base721Error;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    BaseError(#[from] Base721Error),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{reason:?}")]
    BurnNotAllowed {
        reason: String,
    },

    #[error("Health contract should be added to config before burns are allowed")]
    HealthContractNotSet,

    #[error("Credit manager contract should be added to config before burns are allowed")]
    CreditManagerContractNotSet,

    #[error("{0}")]
    Version(#[from] cw2::VersionError),
}
