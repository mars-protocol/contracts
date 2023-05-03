use std::fmt::Display;

use osmosis_test_tube::RunnerError;

pub fn assert_err(actual: RunnerError, expected: impl Display) {
    match actual {
        RunnerError::ExecuteError {
            msg,
        } => {
            println!("ExecuteError, msg: {msg}");
            assert!(msg.contains(&format!("{expected}")))
        }
        RunnerError::QueryError {
            msg,
        } => {
            println!("QueryError, msg: {msg}");
            assert!(msg.contains(&format!("{expected}")))
        }
        _ => panic!("Unhandled error"),
    }
}
