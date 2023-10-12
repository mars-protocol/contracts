use cosmwasm_std::coin;
use cw_it::osmosis_test_tube::{Account, Module, OsmosisTestApp, Wasm};
use mars_owner::OwnerResponse;
use mars_types::swapper::{InstantiateMsg, QueryMsg};

use super::helpers::{instantiate_contract, wasm_file};

#[test]
fn owner_set_on_instantiate() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);
    let signer = app.init_account(&[coin(1_000_000_000_000, "uosmo")]).unwrap();

    let contract_addr = instantiate_contract(&wasm, &signer);

    let res: OwnerResponse = wasm.query(&contract_addr, &QueryMsg::Owner {}).unwrap();
    assert_eq!(res.owner, Some(signer.address()));
}

#[test]
fn raises_on_invalid_owner_addr() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);
    let signer = app.init_account(&[coin(1_000_000_000_000, "uosmo")]).unwrap();

    let wasm_byte_code = std::fs::read(wasm_file()).unwrap();
    let code_id = wasm.store_code(&wasm_byte_code, None, &signer).unwrap().data.code_id;

    let owner = "%%%INVALID%%%";
    let res = wasm.instantiate(
        code_id,
        &InstantiateMsg {
            owner: owner.to_string(),
        },
        None,
        Some("swapper-osmosis-contract"),
        &[],
        &signer,
    );

    if res.is_ok() {
        panic!("Should have thrown an error");
    }
}
