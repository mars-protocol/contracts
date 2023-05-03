use cosmwasm_std::StdError;
use mars_owner::OwnerError;
use thiserror::Error;

pub type ContractResult<T> = Result<T, ContractError>;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("Sent fund mismatch. Expected: {expected:?}, received {received:?}")]
    FundsMismatch {
        expected: String,
        received: String,
    },

    #[error("{0}")]
    OwnerError(#[from] OwnerError),

    #[error("Submessage Reply Error: {0}")]
    ReplyError(String),

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Caller not permitted to perform action")]
    Unauthorized,
}
