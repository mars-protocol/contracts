use crate::helpers::{MockEnv, MAX_VALUE_FOR_BURN};

pub mod helpers;

#[test]
fn instantiated_storage_vars() {
    let mut mock = MockEnv::new()
        .set_minter("spiderman_1337")
        .instantiate_with_health_contract(false)
        .build()
        .unwrap();

    let config = mock.query_config();
    assert_eq!(config.health_contract_addr, None);
    assert_eq!(config.max_value_for_burn, MAX_VALUE_FOR_BURN);

    let ownership = mock.query_ownership();
    assert_eq!("spiderman_1337", ownership.owner.unwrap());
    assert_eq!(None, ownership.pending_owner);

    let next_id = mock.query_next_id();
    assert_eq!(next_id, 1);
}

#[test]
fn instantiated_storage_vars_with_health_contract() {
    let health_contract = "health_contract_xyz_abc";
    let mut mock = MockEnv::new().set_health_contract(health_contract).build().unwrap();

    let config = mock.query_config();
    assert_eq!(config.health_contract_addr, Some(health_contract.to_string()));
    assert_eq!(config.max_value_for_burn, MAX_VALUE_FOR_BURN);

    let next_id = mock.query_next_id();
    assert_eq!(next_id, 1);
}
