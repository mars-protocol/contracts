use cosmwasm_std::{OverflowError, StdError, Uint128};
use cw721_base::ContractError as Base721Error;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    BaseError(#[from] Base721Error),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error(
        "Account balances too high. Collateral + Debts = {current_balances:?}. Max allowed is {max_value_allowed:?}"
    )]
    BurnNotAllowed {
        current_balances: Uint128,
        max_value_allowed: Uint128,
    },
}
