use std::{collections::HashSet, hash::Hash};

use anyhow::Result as AnyResult;
use cw_multi_test::AppResponse;
use mars_params::error::ContractError;

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
    let set_a: HashSet<_> = vec_a.iter().collect();
    let set_b: HashSet<_> = vec_b.iter().collect();

    assert!(set_a == set_b)
}
