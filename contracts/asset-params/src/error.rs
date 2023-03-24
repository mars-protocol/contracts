use cosmwasm_std::StdError;
use mars_owner::OwnerError;
use thiserror::Error;
pub use mars_utils::error::ValidationError;

pub type ContractResult<T> = Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Owner(#[from] OwnerError),

    #[error("Asset is already initialized")]
    AssetAlreadyInitialized {},

    #[error("Asset not initialized")]
    AssetNotInitialized {},

    #[error("{reason:?}")]
    InvalidConfig {
        reason: String,
    },

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("{0}")]
    Validation(#[from] ValidationError),
}
