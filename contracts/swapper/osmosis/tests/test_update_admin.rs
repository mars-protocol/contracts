use cosmwasm_std::coin;
use cw_controllers::{AdminError, AdminResponse};
use osmosis_testing::{Account, Module, OsmosisTestApp, Wasm};

use mars_rover::adapters::swap::{ExecuteMsg, QueryMsg};
use mars_swapper_osmosis::route::OsmosisRoute;

use crate::helpers::{assert_err, instantiate_contract};

pub mod helpers;

#[test]
fn test_only_admin_can_update_admin() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app
        .init_accounts(&[coin(1_000_000_000_000, "uosmo")], 2)
        .unwrap();
    let admin = &accs[0];
    let bad_guy = &accs[1];

    let contract_addr = instantiate_contract(&wasm, admin);

    let res_err = wasm
        .execute(
            &contract_addr,
            &ExecuteMsg::<OsmosisRoute>::UpdateAdmin {
                admin: bad_guy.address(),
            },
            &[],
            bad_guy,
        )
        .unwrap_err();

    assert_err(res_err, AdminError::NotAdmin {});
}

#[test]
fn test_update_admin_works_with_full_config() {
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
        &ExecuteMsg::<OsmosisRoute>::UpdateAdmin {
            admin: new_admin.address(),
        },
        &[],
        admin,
    )
    .unwrap();

    let res: AdminResponse = wasm.query(&contract_addr, &QueryMsg::Admin {}).unwrap();
    assert_eq!(res.admin, Some(new_admin.address()));
}
