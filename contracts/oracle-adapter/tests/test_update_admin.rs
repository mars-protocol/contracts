use cosmwasm_std::Addr;
use cw_controllers_admin_fork::AdminExecuteUpdate;
use cw_multi_test::{App, Executor};

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

    assert!(original_config.admin.is_some());
    assert!(original_config.proposed_new_admin.is_none());
}

#[test]
fn test_propose_new_admin() {
    let mut app = App::default();
    let contract_addr = instantiate_oracle_adapter(&mut app);
    let original_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    let new_admin = "new_admin".to_string();

    // only admin can propose new admins
    let bad_guy = Addr::unchecked("bad_guy");
    app.execute_contract(
        bad_guy.clone(),
        contract_addr.clone(),
        &ExecuteMsg::UpdateAdmin(AdminExecuteUpdate::ProposeNewAdmin {
            proposed: bad_guy.to_string(),
        }),
        &[],
    )
    .unwrap_err();

    app.execute_contract(
        Addr::unchecked(original_config.admin.clone().unwrap()),
        contract_addr.clone(),
        &ExecuteMsg::UpdateAdmin(AdminExecuteUpdate::ProposeNewAdmin {
            proposed: new_admin.clone(),
        }),
        &[],
    )
    .unwrap();

    let new_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    assert_eq!(new_config.admin, original_config.admin);
    assert_ne!(
        new_config.proposed_new_admin,
        original_config.proposed_new_admin
    );
    assert_eq!(new_config.proposed_new_admin, Some(new_admin));
}

#[test]
fn test_clear_proposed() {
    let mut app = App::default();
    let contract_addr = instantiate_oracle_adapter(&mut app);
    let original_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    let new_admin = "new_admin".to_string();

    app.execute_contract(
        Addr::unchecked(original_config.admin.clone().unwrap()),
        contract_addr.clone(),
        &ExecuteMsg::UpdateAdmin(AdminExecuteUpdate::ProposeNewAdmin {
            proposed: new_admin.clone(),
        }),
        &[],
    )
    .unwrap();

    let interim_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    assert_eq!(interim_config.proposed_new_admin, Some(new_admin));

    // only admin can clear
    let bad_guy = Addr::unchecked("bad_guy");
    app.execute_contract(
        bad_guy,
        contract_addr.clone(),
        &ExecuteMsg::UpdateAdmin(AdminExecuteUpdate::ClearProposed),
        &[],
    )
    .unwrap_err();

    app.execute_contract(
        Addr::unchecked(original_config.admin.clone().unwrap()),
        contract_addr.clone(),
        &ExecuteMsg::UpdateAdmin(AdminExecuteUpdate::ClearProposed),
        &[],
    )
    .unwrap();

    let latest_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    assert_eq!(latest_config.admin, original_config.admin);
    assert_ne!(
        latest_config.proposed_new_admin,
        interim_config.proposed_new_admin
    );
    assert_eq!(latest_config.proposed_new_admin, None);
}

#[test]
fn test_accept_admin_role() {
    let mut app = App::default();
    let contract_addr = instantiate_oracle_adapter(&mut app);
    let original_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    let new_admin = "new_admin".to_string();

    app.execute_contract(
        Addr::unchecked(original_config.admin.clone().unwrap()),
        contract_addr.clone(),
        &ExecuteMsg::UpdateAdmin(AdminExecuteUpdate::ProposeNewAdmin {
            proposed: new_admin.clone(),
        }),
        &[],
    )
    .unwrap();

    // Only proposed admin can accept
    app.execute_contract(
        Addr::unchecked(original_config.admin.unwrap()),
        contract_addr.clone(),
        &ExecuteMsg::UpdateAdmin(AdminExecuteUpdate::AcceptProposed),
        &[],
    )
    .unwrap_err();

    app.execute_contract(
        Addr::unchecked(new_admin.clone()),
        contract_addr.clone(),
        &ExecuteMsg::UpdateAdmin(AdminExecuteUpdate::AcceptProposed),
        &[],
    )
    .unwrap();

    let new_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    assert_eq!(new_config.admin.unwrap(), new_admin);
    assert_eq!(new_config.proposed_new_admin, None);
}
