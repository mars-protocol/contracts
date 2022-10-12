use anyhow::Result as AnyResult;
use credit_manager::utils::contents_equal;
use cw_multi_test::AppResponse;
use std::hash::Hash;

use rover::error::ContractError;

pub fn assert_err(res: AnyResult<AppResponse>, err: ContractError) {
    match res {
        Ok(_) => panic!("Result was not an error"),
        Err(generic_err) => {
            let contract_err: ContractError = generic_err.downcast().unwrap();
            assert_eq!(contract_err, err);
        }
    }
}

pub fn assert_contents_equal<T>(vec_a: &[T], vec_b: &[T])
where
    T: Eq + Hash,
{
    assert!(contents_equal(vec_a, vec_b))
}
