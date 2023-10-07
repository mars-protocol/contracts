use crate::helpers::MockEnv;

pub mod helpers;

#[test]
fn owner_set_on_instantiate() {
    let owner = "owner_addr";
    let mock = MockEnv::new().owner(owner).build().unwrap();
    let res = mock.query_config();
    assert_eq!(owner, res.ownership.owner.unwrap());
}

#[test]
#[should_panic]
fn raises_on_invalid_owner_addr() {
    let owner = "%%%INVALID%%%";
    MockEnv::new().owner(owner).params_contract("xyz").health_contract("abc").build().unwrap();
}

#[test]
fn nft_contract_addr_not_set_on_instantiate() {
    let mock = MockEnv::new().no_nft_contract().build().unwrap();
    let res = mock.query_config();
    assert_eq!(res.account_nft, None);
}

#[test]
fn red_bank_set_on_instantiate() {
    let red_bank_addr = "mars_red_bank_contract_123".to_string();
    let mock = MockEnv::new().red_bank(&red_bank_addr).build().unwrap();
    let res = mock.query_config();
    assert_eq!(red_bank_addr, res.red_bank);
}

#[test]
#[should_panic]
fn raises_on_invalid_red_bank_addr() {
    MockEnv::new().red_bank("%%%INVALID%%%").build().unwrap();
}

#[test]
fn oracle_set_on_instantiate() {
    let oracle_contract = "oracle_contract_456".to_string();
    let mock = MockEnv::new().oracle(&oracle_contract).build().unwrap();
    let res = mock.query_config();
    assert_eq!(oracle_contract, res.oracle);
}

#[test]
fn raises_on_invalid_oracle_addr() {
    let mock = MockEnv::new().oracle("%%%INVALID%%%").build();
    if mock.is_ok() {
        panic!("Should have thrown an error");
    }
}

#[test]
fn params_set_on_instantiate() {
    let params_contract = "params_contract_456".to_string();
    let mock = MockEnv::new().params(&params_contract).build().unwrap();
    let res = mock.query_config();
    assert_eq!(params_contract, res.params);
}

#[test]
#[should_panic]
fn raises_on_invalid_params_addr() {
    MockEnv::new().params("%%%INVALID%%%").build().unwrap();
}
