use cosmwasm_std::{CheckedMultiplyRatioError, OverflowError, StdError, Uint128};
use mars_owner::OwnerError;
use mars_types::error::MarsError;
use mars_utils::error::ValidationError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("{0}")]
    Validation(#[from] ValidationError),

    #[error("{0}")]
    Owner(#[from] OwnerError),

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

    #[error("Invalid min receive: {reason}")]
    InvalidMinReceive {
        reason: String,
    },

    #[error("Invalid actions. Only Withdraw and WithdrawLiquidity is possible to pass for CreditManager")]
    InvalidActionsForCreditManager {},

    #[error("{0}")]
    Version(#[from] cw2::VersionError),
}

pub type ContractResult<T> = Result<T, ContractError>;
