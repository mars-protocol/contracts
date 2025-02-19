use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyFractionError, CheckedMultiplyRatioError, OverflowError,
    StdError, Uint128,
};
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

    #[error("{0}")]
    CheckedMultiplyFractionError(#[from] CheckedMultiplyFractionError),

    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

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

    #[error("Min receive given for swap: {denom_in} -> {denom_out} is too small. `min_receive` allowed: {min_receive_minumum}, `min_receive` given: {min_receive_given}")]
    SlippageLimitExceeded {
        denom_in: String,
        denom_out: String,
        min_receive_minumum: Uint128,
        min_receive_given: Uint128,
    },

    #[error("Invalid actions. Only Withdraw and WithdrawLiquidity is possible to pass for CreditManager")]
    InvalidActionsForCreditManager {},

    #[error("{0}")]
    Version(#[from] cw2::VersionError),

    #[error("Unsupported transfer type: {transfer_type}")]
    UnsupportedTransferType {
        transfer_type: String,
    },
}

pub type ContractResult<T> = Result<T, ContractError>;
