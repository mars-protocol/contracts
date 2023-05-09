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

    let ownership = mock.query_ownership();
    assert_eq!(ownership.pending_owner.unwrap(), new_minter);
}

#[test]
fn proposed_minter_can_accept_role() {
    let mut mock = MockEnv::new().build().unwrap();

    let new_minter = Addr::unchecked("new_minter");
    mock.propose_new_minter(&mock.minter.clone(), &new_minter).unwrap();

    mock.accept_proposed_minter(&new_minter).unwrap();

    let ownership = mock.query_ownership();
    if ownership.pending_owner.is_some() {
        panic!("Proposed minter should have been removed from storage");
    }

    assert_eq!(ownership.owner.unwrap().to_string(), new_minter.to_string());

    let res: MinterResponse =
        mock.app.wrap().query_wasm_smart(mock.nft_contract, &QueryMsg::Minter {}).unwrap();

    assert_eq!(res.minter.unwrap(), new_minter.to_string());
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
