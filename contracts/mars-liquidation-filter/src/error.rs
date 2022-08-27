use std::string::FromUtf8Error;

use cosmwasm_std::StdError;
use mars_outpost::error::MarsError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    FromUtf8(#[from] FromUtf8Error),

    #[error("Required coin to be sent: {denom:?}")]
    RequiredCoin {
        denom: String,
    },
}
