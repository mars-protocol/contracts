use cosmwasm_std::{StdError, StdResult};

/// Assert elements in vecs one by one in order to get a more meaningful error
/// when debugging tests
pub fn assert_eq_vec<T: std::fmt::Debug + PartialEq>(expected: Vec<T>, actual: Vec<T>) {
    assert_eq!(expected.len(), actual.len());

    for (i, element) in expected.iter().enumerate() {
        assert_eq!(*element, actual[i]);
    }
}

/// Assert StdError::GenericErr message with expected_msg
pub fn assert_generic_error_message<T>(response: StdResult<T>, expected_msg: &str) {
    match response {
        Err(StdError::GenericErr {
            msg,
            ..
        }) => assert_eq!(msg, expected_msg),
        Err(other_err) => panic!("Unexpected error: {other_err:?}"),
        Ok(_) => panic!("SHOULD NOT ENTER HERE!"),
    }
}
