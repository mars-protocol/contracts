use cosmwasm_std::Addr;
use cw_controllers::AdminError::NotAdmin;
use cw_multi_test::{App, Executor};

use mars_oracle_adapter::error::ContractError::AdminError;
use mars_oracle_adapter::msg::{
    ConfigResponse, ConfigUpdates, ExecuteMsg, QueryMsg, VaultPricingInfo,
};
use mars_rover::adapters::{OracleBase, OracleUnchecked};

use crate::helpers::{assert_err, instantiate_oracle_adapter};

pub mod helpers;

#[test]
fn test_only_admin_can_update_config() {
    let mut app = App::default();
    let contract_addr = instantiate_oracle_adapter(&mut app);

    let bad_guy = Addr::unchecked("bad_guy");
    let res = app.execute_contract(
        bad_guy,
        contract_addr,
        &ExecuteMsg::UpdateConfig {
            new_config: Default::default(),
        },
        &[],
    );

    assert_err(res, AdminError(NotAdmin {}));
}

#[test]
fn test_update_config_works_with_full_config() {
    let mut app = App::default();
    let contract_addr = instantiate_oracle_adapter(&mut app);
    let original_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    let new_owner = Addr::unchecked("new_owner");
    let new_oracle = OracleUnchecked::new("new_oracle".to_string());
    let new_vault_pricing = vec![];

    app.execute_contract(
        original_config.admin.clone().unwrap(),
        contract_addr.clone(),
        &ExecuteMsg::UpdateConfig {
            new_config: ConfigUpdates {
                admin: Some(new_owner.to_string()),
                oracle: Some(new_oracle),
                vault_pricing: Some(new_vault_pricing),
            },
        },
        &[],
    )
    .unwrap();

    let new_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    assert_ne!(new_config.admin, original_config.admin);
    assert_eq!(new_config.admin, Some(new_owner));

    assert_ne!(new_config.oracle, original_config.oracle);
    assert_eq!(
        new_config.oracle,
        OracleBase::new(Addr::unchecked("new_oracle".to_string()))
    );

    let pricing_infos: Vec<VaultPricingInfo> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.to_string(),
            &QueryMsg::AllPricingInfo {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(pricing_infos.len(), 0);
}

#[test]
fn test_update_config_does_nothing_when_nothing_is_passed() {
    let mut app = App::default();
    let contract_addr = instantiate_oracle_adapter(&mut app);
    let original_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    let original_pricing_infos: Vec<VaultPricingInfo> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.to_string(),
            &QueryMsg::AllPricingInfo {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    app.execute_contract(
        original_config.admin.clone().unwrap(),
        contract_addr.clone(),
        &ExecuteMsg::UpdateConfig {
            new_config: ConfigUpdates {
                admin: None,
                oracle: None,
                vault_pricing: None,
            },
        },
        &[],
    )
    .unwrap();

    let new_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(contract_addr.to_string(), &QueryMsg::Config {})
        .unwrap();

    assert_eq!(new_config.admin, original_config.admin);
    assert_eq!(new_config.oracle, original_config.oracle);

    let new_pricing_infos: Vec<VaultPricingInfo> = app
        .wrap()
        .query_wasm_smart(
            contract_addr.to_string(),
            &QueryMsg::AllPricingInfo {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(new_pricing_infos, original_pricing_infos);
}
