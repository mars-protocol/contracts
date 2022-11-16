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

    #[error("Invalid denom. Must be between 3 - 128, got ({len})")]
    InvalidDenomLength {
        len: usize,
    },

    #[error("Expected alphabetic ascii character in denom")]
    InvalidDenomCharacter,

    #[error("Invalid character ({c}) in denom")]
    InvalidCharacter { c: char },

    #[error("Invalid denom: {denom}")]
    InvalidDenom {
        denom: String,
    },
}
