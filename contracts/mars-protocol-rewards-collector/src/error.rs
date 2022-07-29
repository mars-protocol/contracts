use cosmwasm_std::{OverflowError, StdError, Uint128};
use thiserror::Error;

use mars_outpost::error::MarsError;

use osmo_bindings::Step;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("Asset is not enabled for distribution: {denom}")]
    AssetNotEnabledForDistribution {
        denom: String,
    },

    #[error("Amount to distribute {amount} is larger than available balance {balance}")]
    AmountToDistributeTooLarge {
        amount: Uint128,
        balance: Uint128,
    },

    #[error("Invalid swap route {steps:?}: {reason}")]
    InvalidSwapRoute {
        steps: Vec<Step>,
        reason: String,
    }
}

pub type ContractResult<T> = Result<T, ContractError>;
