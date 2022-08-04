use cosmwasm_std::{OverflowError, StdError, Uint128};
use thiserror::Error;

use crate::ConfigError;
use mars_core::error::MarsError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    ConfigError(#[from] ConfigError),

    #[error("Asset is not enabled for distribution: {asset_label:?}")]
    AssetNotEnabled { asset_label: String },

    #[error("Amount to distribute {amount} is larger than available balance {balance}")]
    AmountToDistributeTooLarge { amount: Uint128, balance: Uint128 },
}
