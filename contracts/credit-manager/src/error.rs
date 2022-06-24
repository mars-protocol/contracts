use cosmwasm_std::{Addr, StdError, Uint128};
use cw_asset::AssetListBase;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{user:?} is not authorized to {action:?}")]
    Unauthorized { user: String, action: String },

    #[error("{0} is not whitelisted")]
    NotWhitelisted(String),

    #[error("Extra funds received: {0}")]
    ExtraFundsReceived(AssetListBase<Addr>),

    #[error("Deposits of CW20's should come via Cw20ExecuteMsg::Send to cw20 contract specifying Rover's ReceiveMsg")]
    WrongDepositMethodForCW20 {},

    #[error("Sent fund mismatch. Expected: {expected:?}, received {received:?}")]
    FundsMismatch {
        expected: Uint128,
        received: Uint128,
    },

    #[error("This method cannot be invoked externally")]
    ExternalInvocation {},

    #[error("{user:?} is not the owner of {token_id:?}")]
    NotTokenOwner { user: String, token_id: String },
}
