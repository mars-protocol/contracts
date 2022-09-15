use cosmwasm_std::Addr;
use cw_multi_test::Executor;

use rover::adapters::swap::{Config, ExecuteMsg, QueryMsg};
use rover::error::ContractError as RoverError;
use swapper_base::ContractError;
use swapper_osmosis::route::OsmosisRoute;

use crate::helpers::mock_osmosis_app;
use crate::helpers::{assert_err, instantiate_contract};

pub mod helpers;

#[test]
fn test_only_owner_can_update_config() {
    let mut app = mock_osmosis_app();
    let contract_addr = instantiate_contract(&mut app);

    let bad_guy = Addr::unchecked("bad_guy");
    let res = app.execute_contract(
        bad_guy.clone(),
        contract_addr,
        &ExecuteMsg::<OsmosisRoute>::UpdateConfig {
            owner: Some(bad_guy.to_string()),
        },
        &[],
    );

    assert_err(
        res,
        ContractError::Rover(RoverError::Unauthorized {
            user: bad_guy.to_string(),
            action: "update owner".to_string(),
        }),
    );
}

#[test]
fn test_update_config_works_with_full_config() {
    let owner = Addr::unchecked("owner");
    let mut app = mock_osmosis_app();
    let contract_addr = instantiate_contract(&mut app);

    let new_owner = Addr::unchecked("new_owner");
    app.execute_contract(
        owner.clone(),
        contract_addr.clone(),
        &ExecuteMsg::<OsmosisRoute>::UpdateConfig {
            owner: Some(new_owner.to_string()),
        },
        &[],
    )
    .unwrap();

    let new_config: Config<String> = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    assert_ne!(new_config.owner, owner.to_string());
    assert_eq!(new_config.owner, new_owner.to_string());
}

#[test]
fn test_update_config_does_nothing_when_nothing_is_passed() {
    let owner = Addr::unchecked("owner");
    let mut app = mock_osmosis_app();
    let contract_addr = instantiate_contract(&mut app);

    app.execute_contract(
        owner.clone(),
        contract_addr.clone(),
        &ExecuteMsg::<OsmosisRoute>::UpdateConfig { owner: None },
        &[],
    )
    .unwrap();

    let new_config: Config<String> = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    assert_eq!(new_config.owner, owner.to_string());
}
