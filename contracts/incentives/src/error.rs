use std::string::FromUtf8Error;

use cosmwasm_std::{Coin, OverflowError, StdError};
use mars_owner::OwnerError;
use mars_types::error::MarsError;
use mars_utils::error::{GuardError, ValidationError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("{0}")]
    Validation(#[from] ValidationError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    FromUtf8(#[from] FromUtf8Error),

    #[error("{0}")]
    Owner(#[from] OwnerError),

    #[error("Invalid incentive: {reason}")]
    InvalidIncentive {
        reason: String,
    },

    #[error("Invalid Pagination Params. If start_after_incentive_denom is supplied, then start_after_collateral_denom must also be supplied")]
    InvalidPaginationParams,

    #[error("Invalid duration. Incentive duration must be divisible by epoch duration. Epoch duration is {epoch_duration}")]
    InvalidDuration {
        epoch_duration: u64,
    },

    #[error("Invalid start time. Incentive start time must be a multiple of epoch duration away from an existing schedule. Epoch duration is {epoch_duration}. Existing start time is {existing_start_time}")]
    InvalidStartTime {
        existing_start_time: u64,
        epoch_duration: u64,
    },

    #[error("Invalid funds. Expected {expected} funds")]
    InvalidFunds {
        expected: Coin,
    },

    #[error("Invalid incentive denom. {denom} is not whitelisted")]
    NotWhitelisted {
        denom: String,
    },

    #[error("Max whitelist limit reached. Max whitelist limit is {max_whitelist_limit}")]
    MaxWhitelistLimitReached {
        max_whitelist_limit: u8,
    },

    #[error("Epoch duration too short. Minimum epoch duration is {min_epoch_duration}")]
    EpochDurationTooShort {
        min_epoch_duration: u64,
    },

    #[error("Whitelist update arguments contain duplicate denom")]
    DuplicateDenom {
        denom: String,
    },

    #[error("{0}")]
    Version(#[from] cw2::VersionError),

    #[error("{0}")]
    Guard(#[from] GuardError),

    #[error("Account id {account_id} has no staked LP position for denom: {denom}")]
    NoStakedLp {
        account_id: String,
        denom: String,
    },

    #[error("No deposits for {denom} exist")]
    NoDeposits {
        denom: String,
    },
}

impl From<ContractError> for StdError {
    fn from(err: ContractError) -> Self {
        StdError::generic_err(err.to_string())
    }
}
