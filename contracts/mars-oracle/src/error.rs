use cosmwasm_std::{OverflowError, StdError};
use mars_core::error::MarsError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("Price source is not TWAP")]
    PriceSourceNotTwap {},

    #[error("Native price not found")]
    NativePriceNotFound {},

    #[error("No TWAP snapshot within tolerance")]
    NoSnapshotWithinTolerance {},

    #[error("Invalid pair")]
    InvalidPair {},
}

impl From<ContractError> for StdError {
    fn from(source: ContractError) -> Self {
        match source {
            ContractError::Std(e) => e,
            e => StdError::generic_err(format!("{}", e)),
        }
    }
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_error_to_std_error() {
        {
            let contract_error = ContractError::Mars(MarsError::Unauthorized {});

            let std_error: StdError = contract_error.into();

            assert_eq!(std_error, StdError::generic_err("Unauthorized"))
        }

        {
            let contract_error = ContractError::Std(StdError::generic_err("Some error"));

            let std_error: StdError = contract_error.into();

            assert_eq!(std_error, StdError::generic_err("Some error"))
        }

        {
            let contract_error = ContractError::PriceSourceNotTwap {};

            let std_error: StdError = contract_error.into();

            assert_eq!(std_error, StdError::generic_err("Price source is not TWAP"))
        }
    }
}
