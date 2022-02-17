use thiserror::Error;

use cosmwasm_std::{OverflowError, StdError};

use mars_core::error::MarsError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("Stake amount must be greater than 0")]
    StakeAmountZero {},

    #[error("Unstake amount must be greater than 0")]
    UnstakeAmountZero {},

    #[error("Cannot unstake if address has an active claim")]
    UnstakeActiveClaim {},

    #[error("Total MARS being claimed cannot be greater than staking contract's balance")]
    MarsForClaimersOverflow {},

    #[error("Cooldown has not ended")]
    ClaimCooldownNotEnded {},

    #[error("Mars amount to transfer is greater than total balance")]
    TransferMarsAmountTooLarge {},

    #[error("Cannot have two slash events on the same block")]
    TransferMarsCannotHaveTwoSlashEventsOnBlock {},

    #[error("Cannot swap MARS")]
    MarsCannotSwap {},
}
