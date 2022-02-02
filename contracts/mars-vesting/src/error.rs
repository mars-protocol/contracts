use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

use mars_core::error::MarsError;
use mars_core::math::decimal::Decimal;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("Only Mars token can be deposited")]
    InvalidTokenDeposit {},

    #[error("Data already exists for user: {user_address}")]
    DataAlreadyExists { user_address: String },

    #[error("Cannot find attribute: {key}")]
    ReplyParseFailed { key: String },

    #[error("Mars/xMars ratio is undefined")]
    XMarsRatioUndefined {},

    #[error("Unlock time setup is invalid")]
    InvalidUnlockTimeSetup {},

    #[error("Mars:XMars ratio is not one, is {xmars_per_mars} xMARS per MARS")]
    MarsXMarsRatioNotOne { xmars_per_mars: Decimal },
}
