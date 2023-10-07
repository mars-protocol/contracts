use cosmwasm_std::{OverflowError, StdError};
use cw_dex::CwDexError;
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("{0}")]
    CwDexError(#[from] CwDexError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("{0}")]
    Version(#[from] cw2::VersionError),
}
