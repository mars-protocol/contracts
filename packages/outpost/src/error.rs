use cosmwasm_std::StdError;
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

    #[error("Invalid param: {param_name} is {invalid_value}, but it should be {predicate}")]
    InvalidParam {
        param_name: String,
        invalid_value: String,
        predicate: String,
    },

    #[error("Failed to deserialize RPC query response for: {target_type}")]
    Deserialize {
        target_type: String,
    },

    #[error("Invalid denom: {reason}")]
    InvalidDenom {
        reason: String,
    },
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
    fn test_mars_error_to_std_error() {
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
