use cosmwasm_std::{CheckedMultiplyRatioError, StdError, Uint128};
use thiserror::Error;

use crate::coin_list::CoinList;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    CheckedMultiply(#[from] CheckedMultiplyRatioError),

    #[error("{user:?} is not authorized to {action:?}")]
    Unauthorized { user: String, action: String },

    #[error("{0} is not whitelisted")]
    NotWhitelisted(String),

    #[error("Extra funds received: {0}")]
    ExtraFundsReceived(CoinList),

    #[error("No coin amount set for action")]
    NoAmount,

    #[error("Sent fund mismatch. Expected: {expected:?}, received {received:?}")]
    FundsMismatch {
        expected: Uint128,
        received: Uint128,
    },

    #[error("Callbacks cannot be invoked externally")]
    ExternalInvocation,

    #[error("{user:?} is not the owner of {token_id:?}")]
    NotTokenOwner { user: String, token_id: String },
}
