use crate::helpers::{MockEnv, MAX_VALUE_FOR_BURN};

pub mod helpers;

#[test]
fn test_storage_vars_set_on_instantiate() {
    let mut mock = MockEnv::new().build().unwrap();

    let config = mock.query_config();
    assert_eq!(config.proposed_new_minter, None);
    assert_eq!(config.max_value_for_burn, MAX_VALUE_FOR_BURN);

    let next_id = mock.query_next_id();
    assert_eq!(next_id, 1);
}
