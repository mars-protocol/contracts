use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyFractionError, DivideByZeroError, OverflowError, StdError,
};
use mars_utils::error::ValidationError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum MarsError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("All params should be available during instantiation")]
    InstantiateParamsUnavailable {},

    #[error("Incorrect number of addresses, expected {expected:?}, got {actual:?}")]
    AddressesQueryWrongNumber {
        expected: u32,
        actual: u32,
    },

    #[error("Failed to deserialize RPC query response for: {target_type}")]
    Deserialize {
        target_type: String,
    },

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    DivideByZero(#[from] DivideByZeroError),

    #[error("{0}")]
    CheckedFromRatio(#[from] CheckedFromRatioError),

    #[error("{0}")]
    CheckedMultiplyFraction(#[from] CheckedMultiplyFractionError),

    #[error("{0}")]
    Validation(#[from] ValidationError),
}

impl From<MarsError> for StdError {
    fn from(source: MarsError) -> Self {
        match source {
            MarsError::Std(e) => e,
            e => StdError::generic_err(e.to_string()),
        }
    }
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::MarsError;

    #[test]
    fn mars_error_to_std_error() {
        {
            let mars_error = MarsError::Unauthorized {};

            let std_error: StdError = mars_error.into();

            assert_eq!(std_error, StdError::generic_err("Unauthorized"))
        }

        {
            let mars_error = MarsError::Std(StdError::generic_err("Some error"));

            let std_error: StdError = mars_error.into();

            assert_eq!(std_error, StdError::generic_err("Some error"))
        }

        {
            let mars_error = MarsError::Std(StdError::invalid_data_size(1, 2));

            let std_error: StdError = mars_error.into();

            assert_eq!(std_error, StdError::invalid_data_size(1, 2))
        }
    }
}
