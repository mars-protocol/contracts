use cosmwasm_std::coin;
use osmosis_testing::{Account, Module, OsmosisTestApp, Wasm};

use cw_controllers_admin_fork::{AdminResponse, AdminUpdate};
use mars_rover::adapters::swap::{ExecuteMsg, QueryMsg};
use mars_swapper_osmosis::route::OsmosisRoute;

use crate::helpers::instantiate_contract;

pub mod helpers;

#[test]
fn test_initial_state() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app
        .init_accounts(&[coin(1_000_000_000_000, "uosmo")], 2)
        .unwrap();
    let admin = &accs[0];

    let contract_addr = instantiate_contract(&wasm, admin);

    let res: AdminResponse = wasm.query(&contract_addr, &QueryMsg::Admin {}).unwrap();
    assert_eq!(res.admin.unwrap(), admin.address());
    assert_eq!(res.proposed, None);
}

#[test]
fn test_only_admin_can_propose() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app
        .init_accounts(&[coin(1_000_000_000_000, "uosmo")], 3)
        .unwrap();
    let admin = &accs[0];
    let bad_guy = &accs[1];

    let contract_addr = instantiate_contract(&wasm, admin);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateAdmin(AdminUpdate::ProposeNewAdmin {
            proposed: bad_guy.address(),
        }),
        &[],
        bad_guy,
    )
    .unwrap_err();
}

#[test]
fn test_propose_new_admin() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app
        .init_accounts(&[coin(1_000_000_000_000, "uosmo")], 2)
        .unwrap();
    let admin = &accs[0];
    let new_admin = &accs[1];

    let contract_addr = instantiate_contract(&wasm, admin);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateAdmin(AdminUpdate::ProposeNewAdmin {
            proposed: new_admin.address(),
        }),
        &[],
        admin,
    )
    .unwrap();

    let res: AdminResponse = wasm.query(&contract_addr, &QueryMsg::Admin {}).unwrap();
    assert_eq!(res.admin.unwrap(), admin.address());
    assert_eq!(res.proposed.unwrap(), new_admin.address());
}

#[test]
fn test_only_admin_can_clear_proposed() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app
        .init_accounts(&[coin(1_000_000_000_000, "uosmo")], 3)
        .unwrap();
    let admin = &accs[0];
    let bad_guy = &accs[1];
    let new_admin = &accs[2];

    let contract_addr = instantiate_contract(&wasm, admin);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateAdmin(AdminUpdate::ProposeNewAdmin {
            proposed: new_admin.address(),
        }),
        &[],
        admin,
    )
    .unwrap();

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateAdmin(AdminUpdate::ClearProposed),
        &[],
        bad_guy,
    )
    .unwrap_err();
}

#[test]
fn test_clear_proposed() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app
        .init_accounts(&[coin(1_000_000_000_000, "uosmo")], 2)
        .unwrap();
    let admin = &accs[0];
    let new_admin = &accs[1];

    let contract_addr = instantiate_contract(&wasm, admin);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateAdmin(AdminUpdate::ProposeNewAdmin {
            proposed: new_admin.address(),
        }),
        &[],
        admin,
    )
    .unwrap();

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateAdmin(AdminUpdate::ClearProposed),
        &[],
        admin,
    )
    .unwrap();

    let res: AdminResponse = wasm.query(&contract_addr, &QueryMsg::Admin {}).unwrap();
    assert_eq!(res.admin.unwrap(), admin.address());
    assert_eq!(res.proposed, None);
}

#[test]
fn test_only_proposed_admin_can_accept_role() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app
        .init_accounts(&[coin(1_000_000_000_000, "uosmo")], 2)
        .unwrap();
    let admin = &accs[0];
    let new_admin = &accs[1];

    let contract_addr = instantiate_contract(&wasm, admin);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateAdmin(AdminUpdate::ProposeNewAdmin {
            proposed: new_admin.address(),
        }),
        &[],
        admin,
    )
    .unwrap();

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateAdmin(AdminUpdate::AcceptProposed),
        &[],
        admin,
    )
    .unwrap_err();
}

#[test]
fn test_accept_admin_role() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app
        .init_accounts(&[coin(1_000_000_000_000, "uosmo")], 2)
        .unwrap();
    let admin = &accs[0];
    let new_admin = &accs[1];

    let contract_addr = instantiate_contract(&wasm, admin);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateAdmin(AdminUpdate::ProposeNewAdmin {
            proposed: new_admin.address(),
        }),
        &[],
        admin,
    )
    .unwrap();

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateAdmin(AdminUpdate::AcceptProposed),
        &[],
        new_admin,
    )
    .unwrap();

    let res: AdminResponse = wasm.query(&contract_addr, &QueryMsg::Admin {}).unwrap();
    assert_eq!(res.admin.unwrap(), new_admin.address());
    assert_eq!(res.proposed, None);
}
