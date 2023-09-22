use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ValidationError {
    #[error("Invalid param: {param_name} is {invalid_value}, but it should be {predicate}")]
    InvalidParam {
        param_name: String,
        invalid_value: String,
        predicate: String,
    },

    #[error("Invalid denom: {reason}")]
    InvalidDenom {
        reason: String,
    },
}

#[derive(Error, Debug, PartialEq)]
pub enum GuardError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Guard is active")]
    Active {},

    #[error("Guard is inactive")]
    Inactive {},

    #[error("Invalid guard state transition")]
    InvalidState {},
}
