use cosmwasm_std::Addr;
use cw_multi_test::Executor;

use rover::adapters::swap::{Config, InstantiateMsg, QueryMsg};

use crate::helpers::{mock_osmosis_app, mock_osmosis_contract};

pub mod helpers;

#[test]
fn test_owner_set_on_instantiate() {
    let owner = Addr::unchecked("owner");
    let mut app = mock_osmosis_app();
    let contract = mock_osmosis_contract();
    let code_id = app.store_code(contract);
    let contract_addr = app
        .instantiate_contract(
            code_id,
            owner.clone(),
            &InstantiateMsg {
                owner: owner.to_string(),
            },
            &[],
            "mock-swapper-contract",
            None,
        )
        .unwrap();

    let config: Config<String> = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    assert_eq!(config.owner, owner);
}

#[test]
fn test_raises_on_invalid_owner_addr() {
    let owner = "%%%INVALID%%%";
    let mut app = mock_osmosis_app();
    let contract = mock_osmosis_contract();
    let code_id = app.store_code(contract);
    let res = app.instantiate_contract(
        code_id,
        Addr::unchecked(owner),
        &InstantiateMsg {
            owner: owner.to_string(),
        },
        &[],
        "mock-swapper-contract",
        None,
    );

    if res.is_ok() {
        panic!("Should have thrown an error");
    }
}
