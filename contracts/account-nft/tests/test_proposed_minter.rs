use cosmwasm_std::Addr;
use cw721_base::MinterResponse;
use mars_account_nft::msg::QueryMsg;

use crate::helpers::MockEnv;

pub mod helpers;

#[test]
fn only_minter_can_propose_new_minter() {
    let mut mock = MockEnv::new().build().unwrap();

    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.propose_new_minter(&bad_guy, &bad_guy);

    if res.is_ok() {
        panic!("Non-minter should not be able to propose new minter");
    }
}

#[test]
fn propose_minter_stores() {
    let mut mock = MockEnv::new().build().unwrap();

    let new_minter = Addr::unchecked("new_minter");
    mock.propose_new_minter(&mock.minter.clone(), &new_minter).unwrap();

    let config = mock.query_config();
    assert_eq!(config.proposed_new_minter.unwrap(), new_minter);
}

#[test]
fn proposed_minter_can_accept_role() {
    let mut mock = MockEnv::new().build().unwrap();

    let new_minter = Addr::unchecked("new_minter");
    mock.propose_new_minter(&mock.minter.clone(), &new_minter).unwrap();

    mock.accept_proposed_minter(&new_minter).unwrap();

    let config = mock.query_config();
    if config.proposed_new_minter.is_some() {
        panic!("Proposed minter should have been removed from storage");
    }

    let res: MinterResponse =
        mock.app.wrap().query_wasm_smart(mock.nft_contract, &QueryMsg::Minter {}).unwrap();

    assert_eq!(res.minter, new_minter)
}

#[test]
fn only_proposed_minter_can_accept() {
    let mut mock = MockEnv::new().build().unwrap();

    let new_minter = Addr::unchecked("new_minter");
    mock.propose_new_minter(&mock.minter.clone(), &new_minter).unwrap();

    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.accept_proposed_minter(&bad_guy);

    if res.is_ok() {
        panic!("Only proposed minter can accept role");
    }
}
