use anyhow::Result as AnyResult;
use cw_multi_test::AppResponse;

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

pub fn assert_contents_equal<T: PartialEq>(vec_a: Vec<T>, vec_b: Vec<T>) {
    assert_eq!(vec_a.len(), vec_b.len());
    assert!(vec_a.iter().all(|item| vec_b.contains(item)));
    assert!(vec_b.iter().all(|item| vec_a.contains(item)));
}
