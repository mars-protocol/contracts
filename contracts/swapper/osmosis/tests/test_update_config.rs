use cosmwasm_std::coin;
use osmosis_testing::{Account, Module, OsmosisTestApp, Wasm};

use mars_rover::adapters::swap::{Config, ExecuteMsg, QueryMsg};
use mars_rover::error::ContractError as RoverError;
use mars_swapper_base::ContractError;
use mars_swapper_osmosis::route::OsmosisRoute;

use crate::helpers::{assert_err, instantiate_contract};

pub mod helpers;

#[test]
fn test_only_owner_can_update_config() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app
        .init_accounts(&[coin(1_000_000_000_000, "uosmo")], 2)
        .unwrap();
    let owner = &accs[0];
    let bad_guy = &accs[1];

    let contract_addr = instantiate_contract(&wasm, owner);

    let res_err = wasm
        .execute(
            &contract_addr,
            &ExecuteMsg::<OsmosisRoute>::UpdateConfig {
                owner: Some(bad_guy.address()),
            },
            &[],
            bad_guy,
        )
        .unwrap_err();

    assert_err(
        res_err,
        ContractError::Rover(RoverError::Unauthorized {
            user: bad_guy.address(),
            action: "update owner".to_string(),
        }),
    );
}

#[test]
fn test_update_config_works_with_full_config() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);

    let accs = app
        .init_accounts(&[coin(1_000_000_000_000, "uosmo")], 2)
        .unwrap();
    let owner = &accs[0];
    let new_owner = &accs[1];

    let contract_addr = instantiate_contract(&wasm, owner);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateConfig {
            owner: Some(new_owner.address()),
        },
        &[],
        owner,
    )
    .unwrap();

    let config: Config<String> = wasm.query(&contract_addr, &QueryMsg::Config {}).unwrap();
    assert_eq!(config.owner, new_owner.address());
}

#[test]
fn test_update_config_does_nothing_when_nothing_is_passed() {
    let app = OsmosisTestApp::new();
    let wasm = Wasm::new(&app);
    let owner = app
        .init_account(&[coin(1_000_000_000_000, "uosmo")])
        .unwrap();

    let contract_addr = instantiate_contract(&wasm, &owner);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateConfig { owner: None },
        &[],
        &owner,
    )
    .unwrap();

    let config: Config<String> = wasm.query(&contract_addr, &QueryMsg::Config {}).unwrap();
    assert_eq!(config.owner, owner.address());
}
