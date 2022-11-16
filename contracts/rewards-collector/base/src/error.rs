use cosmwasm_std::{CheckedMultiplyRatioError, OverflowError, StdError, Uint128};
use thiserror::Error;

use mars_outpost::error::MarsError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    CheckedMultiplyRatio(#[from] CheckedMultiplyRatioError),

    #[error("Asset is not enabled for distribution: {denom}")]
    AssetNotEnabledForDistribution {
        denom: String,
    },

    #[error("Amount to distribute {amount} is larger than available balance {balance}")]
    AmountToDistributeTooLarge {
        amount: Uint128,
        balance: Uint128,
    },

    #[error("Invalid route: {reason}")]
    InvalidRoute {
        reason: String,
    },

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

pub type ContractResult<T> = Result<T, ContractError>;
