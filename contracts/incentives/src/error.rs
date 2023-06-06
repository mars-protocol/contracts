use std::string::FromUtf8Error;

use cosmwasm_std::StdError;
use mars_owner::OwnerError;
use mars_red_bank_types::error::MarsError;
use mars_utils::error::ValidationError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("{0}")]
    Validation(#[from] ValidationError),

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
}

impl From<ContractError> for StdError {
    fn from(err: ContractError) -> Self {
        StdError::generic_err(err.to_string())
    }
}
