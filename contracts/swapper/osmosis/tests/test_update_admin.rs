use cosmwasm_std::coin;
use mars_owner::{OwnerResponse, OwnerUpdate};
use mars_rover::adapters::swap::{ExecuteMsg, QueryMsg};
use mars_swapper_osmosis::route::OsmosisRoute;
use osmosis_testing::{Account, Module, OsmosisTestApp, Wasm};

use crate::helpers::instantiate_contract;

pub mod helpers;

#[test]
fn initial_state() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app.init_accounts(&[coin(1_000_000_000_000, "uosmo")], 2).unwrap();
    let owner = &accs[0];

    let contract_addr = instantiate_contract(&wasm, owner);

    let res: OwnerResponse = wasm.query(&contract_addr, &QueryMsg::Owner {}).unwrap();
    assert_eq!(res.owner.unwrap(), owner.address());
    assert_eq!(res.proposed, None);
}

#[test]
fn only_owner_can_propose() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app.init_accounts(&[coin(1_000_000_000_000, "uosmo")], 3).unwrap();
    let owner = &accs[0];
    let bad_guy = &accs[1];

    let contract_addr = instantiate_contract(&wasm, owner);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateOwner(OwnerUpdate::ProposeNewOwner {
            proposed: bad_guy.address(),
        }),
        &[],
        bad_guy,
    )
    .unwrap_err();
}

#[test]
fn propose_new_owner() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app.init_accounts(&[coin(1_000_000_000_000, "uosmo")], 2).unwrap();
    let owner = &accs[0];
    let new_owner = &accs[1];

    let contract_addr = instantiate_contract(&wasm, owner);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateOwner(OwnerUpdate::ProposeNewOwner {
            proposed: new_owner.address(),
        }),
        &[],
        owner,
    )
    .unwrap();

    let res: OwnerResponse = wasm.query(&contract_addr, &QueryMsg::Owner {}).unwrap();
    assert_eq!(res.owner.unwrap(), owner.address());
    assert_eq!(res.proposed.unwrap(), new_owner.address());
}

#[test]
fn only_owner_can_clear_proposed() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app.init_accounts(&[coin(1_000_000_000_000, "uosmo")], 3).unwrap();
    let owner = &accs[0];
    let bad_guy = &accs[1];
    let new_owner = &accs[2];

    let contract_addr = instantiate_contract(&wasm, owner);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateOwner(OwnerUpdate::ProposeNewOwner {
            proposed: new_owner.address(),
        }),
        &[],
        owner,
    )
    .unwrap();

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateOwner(OwnerUpdate::ClearProposed),
        &[],
        bad_guy,
    )
    .unwrap_err();
}

#[test]
fn clear_proposed() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app.init_accounts(&[coin(1_000_000_000_000, "uosmo")], 2).unwrap();
    let owner = &accs[0];
    let new_owner = &accs[1];

    let contract_addr = instantiate_contract(&wasm, owner);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateOwner(OwnerUpdate::ProposeNewOwner {
            proposed: new_owner.address(),
        }),
        &[],
        owner,
    )
    .unwrap();

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateOwner(OwnerUpdate::ClearProposed),
        &[],
        owner,
    )
    .unwrap();

    let res: OwnerResponse = wasm.query(&contract_addr, &QueryMsg::Owner {}).unwrap();
    assert_eq!(res.owner.unwrap(), owner.address());
    assert_eq!(res.proposed, None);
}

#[test]
fn only_proposed_owner_can_accept_role() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app.init_accounts(&[coin(1_000_000_000_000, "uosmo")], 2).unwrap();
    let owner = &accs[0];
    let new_owner = &accs[1];

    let contract_addr = instantiate_contract(&wasm, owner);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateOwner(OwnerUpdate::ProposeNewOwner {
            proposed: new_owner.address(),
        }),
        &[],
        owner,
    )
    .unwrap();

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateOwner(OwnerUpdate::AcceptProposed),
        &[],
        owner,
    )
    .unwrap_err();
}

#[test]
fn accept_owner_role() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app.init_accounts(&[coin(1_000_000_000_000, "uosmo")], 2).unwrap();
    let owner = &accs[0];
    let new_owner = &accs[1];

    let contract_addr = instantiate_contract(&wasm, owner);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateOwner(OwnerUpdate::ProposeNewOwner {
            proposed: new_owner.address(),
        }),
        &[],
        owner,
    )
    .unwrap();

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateOwner(OwnerUpdate::AcceptProposed),
        &[],
        new_owner,
    )
    .unwrap();

    let res: OwnerResponse = wasm.query(&contract_addr, &QueryMsg::Owner {}).unwrap();
    assert_eq!(res.owner.unwrap(), new_owner.address());
    assert_eq!(res.proposed, None);
}
