use cosmwasm_std::Addr;
use cw_multi_test::{App, Executor};
use mars_owner::OwnerUpdate;

use mars_oracle_adapter::msg::{ConfigResponse, ExecuteMsg, QueryMsg};

use crate::helpers::instantiate_oracle_adapter;

pub mod helpers;

#[test]
fn test_initialized_state() {
    let mut app = App::default();
    let contract_addr = instantiate_oracle_adapter(&mut app);
    let original_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    assert!(original_config.owner.is_some());
    assert!(original_config.proposed_new_owner.is_none());
}

#[test]
fn test_propose_new_owner() {
    let mut app = App::default();
    let contract_addr = instantiate_oracle_adapter(&mut app);
    let original_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    let new_owner = "new_owner".to_string();

    // only owner can propose new owners
    let bad_guy = Addr::unchecked("bad_guy");
    app.execute_contract(
        bad_guy.clone(),
        contract_addr.clone(),
        &ExecuteMsg::UpdateOwner(OwnerUpdate::ProposeNewOwner {
            proposed: bad_guy.to_string(),
        }),
        &[],
    )
    .unwrap_err();

    app.execute_contract(
        Addr::unchecked(original_config.owner.clone().unwrap()),
        contract_addr.clone(),
        &ExecuteMsg::UpdateOwner(OwnerUpdate::ProposeNewOwner {
            proposed: new_owner.clone(),
        }),
        &[],
    )
    .unwrap();

    let new_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    assert_eq!(new_config.owner, original_config.owner);
    assert_ne!(
        new_config.proposed_new_owner,
        original_config.proposed_new_owner
    );
    assert_eq!(new_config.proposed_new_owner, Some(new_owner));
}

#[test]
fn test_clear_proposed() {
    let mut app = App::default();
    let contract_addr = instantiate_oracle_adapter(&mut app);
    let original_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    let new_owner = "new_owner".to_string();

    app.execute_contract(
        Addr::unchecked(original_config.owner.clone().unwrap()),
        contract_addr.clone(),
        &ExecuteMsg::UpdateOwner(OwnerUpdate::ProposeNewOwner {
            proposed: new_owner.clone(),
        }),
        &[],
    )
    .unwrap();

    let interim_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    assert_eq!(interim_config.proposed_new_owner, Some(new_owner));

    // only owner can clear
    let bad_guy = Addr::unchecked("bad_guy");
    app.execute_contract(
        bad_guy,
        contract_addr.clone(),
        &ExecuteMsg::UpdateOwner(OwnerUpdate::ClearProposed),
        &[],
    )
    .unwrap_err();

    app.execute_contract(
        Addr::unchecked(original_config.owner.clone().unwrap()),
        contract_addr.clone(),
        &ExecuteMsg::UpdateOwner(OwnerUpdate::ClearProposed),
        &[],
    )
    .unwrap();

    let latest_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    assert_eq!(latest_config.owner, original_config.owner);
    assert_ne!(
        latest_config.proposed_new_owner,
        interim_config.proposed_new_owner
    );
    assert_eq!(latest_config.proposed_new_owner, None);
}

#[test]
fn test_accept_owner_role() {
    let mut app = App::default();
    let contract_addr = instantiate_oracle_adapter(&mut app);
    let original_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    let new_owner = "new_owner".to_string();

    app.execute_contract(
        Addr::unchecked(original_config.owner.clone().unwrap()),
        contract_addr.clone(),
        &ExecuteMsg::UpdateOwner(OwnerUpdate::ProposeNewOwner {
            proposed: new_owner.clone(),
        }),
        &[],
    )
    .unwrap();

    // Only proposed owner can accept
    app.execute_contract(
        Addr::unchecked(original_config.owner.unwrap()),
        contract_addr.clone(),
        &ExecuteMsg::UpdateOwner(OwnerUpdate::AcceptProposed),
        &[],
    )
    .unwrap_err();

    app.execute_contract(
        Addr::unchecked(new_owner.clone()),
        contract_addr.clone(),
        &ExecuteMsg::UpdateOwner(OwnerUpdate::AcceptProposed),
        &[],
    )
    .unwrap();

    let new_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    assert_eq!(new_config.owner.unwrap(), new_owner);
    assert_eq!(new_config.proposed_new_owner, None);
}
