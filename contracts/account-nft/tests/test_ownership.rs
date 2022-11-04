use anyhow::Result as AnyResult;
use cosmwasm_std::{Addr, StdResult};
use cw721_base::MinterResponse;
use cw_multi_test::{App, AppResponse, BasicApp, Executor};

use mars_account_nft::msg::{ExecuteMsg as ExtendedExecuteMsg, QueryMsg};

use crate::helpers::instantiate_mock_nft_contract;

pub mod helpers;

#[test]
fn test_only_owner_can_propose_ownership_transfer() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let contract_addr = instantiate_mock_nft_contract(&mut app, &owner);

    let bad_guy = Addr::unchecked("bad_guy");
    let res = propose_new_owner(&mut app, &contract_addr, &bad_guy, &bad_guy);

    if res.is_ok() {
        panic!("Non-owner should not be able to propose ownership transfer");
    }
}

#[test]
fn test_propose_ownership_stores() {
    let mut app = App::default();
    let original_owner = Addr::unchecked("owner");
    let contract_addr = instantiate_mock_nft_contract(&mut app, &original_owner);

    let new_owner = Addr::unchecked("new_owner");
    propose_new_owner(&mut app, &contract_addr, &original_owner, &new_owner).unwrap();

    let pending_owner_in_storage = query_pending_owner(&app, &contract_addr).unwrap();
    assert_eq!(pending_owner_in_storage, new_owner);
}

#[test]
fn test_proposed_owner_can_accept_ownership() {
    let mut app = App::default();
    let original_owner = Addr::unchecked("owner");
    let contract_addr = instantiate_mock_nft_contract(&mut app, &original_owner);

    let new_owner = Addr::unchecked("new_owner");
    propose_new_owner(&mut app, &contract_addr, &original_owner, &new_owner).unwrap();

    accept_proposed_owner(&mut app, &contract_addr, &new_owner).unwrap();

    let res = query_pending_owner(&app, &contract_addr);
    if res.is_ok() {
        panic!("Proposed owner should have been removed from storage");
    }

    let res: MinterResponse = app
        .wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::Minter {})
        .unwrap();

    assert_eq!(res.minter, new_owner)
}

#[test]
fn test_only_proposed_owner_can_accept() {
    let mut app = App::default();
    let original_owner = Addr::unchecked("owner");
    let contract_addr = instantiate_mock_nft_contract(&mut app, &original_owner);

    let new_owner = Addr::unchecked("new_owner");
    propose_new_owner(&mut app, &contract_addr, &original_owner, &new_owner).unwrap();

    let bad_guy = Addr::unchecked("bad_guy");
    let res = accept_proposed_owner(&mut app, &contract_addr, &bad_guy);
    if res.is_ok() {
        panic!("Only proposed owner can accept ownership");
    }
}

fn query_pending_owner(app: &BasicApp, contract_addr: &Addr) -> StdResult<String> {
    app.wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::ProposedNewOwner {})
}

fn propose_new_owner(
    app: &mut BasicApp,
    contract_addr: &Addr,
    sender: &Addr,
    proposed_new_owner: &Addr,
) -> AnyResult<AppResponse> {
    app.execute_contract(
        sender.clone(),
        contract_addr.clone(),
        &ExtendedExecuteMsg::ProposeNewOwner {
            new_owner: proposed_new_owner.into(),
        },
        &[],
    )
}

fn accept_proposed_owner(
    app: &mut BasicApp,
    contract_addr: &Addr,
    sender: &Addr,
) -> AnyResult<AppResponse> {
    app.execute_contract(
        sender.clone(),
        contract_addr.clone(),
        &ExtendedExecuteMsg::AcceptOwnership {},
        &[],
    )
}
