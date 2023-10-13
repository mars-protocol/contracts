use std::str::FromStr;

use cosmwasm_std::{Addr, Decimal};
use mars_owner::OwnerError;
use mars_params::error::ContractError::Owner;
use mars_types::params::{VaultConfigUnchecked, VaultConfigUpdate};

use super::helpers::{assert_contents_equal, assert_err, default_vault_config, MockEnv};

#[test]
fn initial_state_of_vault_configs() {
    let mock = MockEnv::new().build().unwrap();
    let configs = mock.query_all_vault_configs(None, None);
    assert!(configs.is_empty());
}

#[test]
fn only_owner_can_update_vault_configs() {
    let mut mock = MockEnv::new().build().unwrap();
    let bad_guy = Addr::unchecked("doctor_otto_983");
    let res = mock.update_vault_config(
        &bad_guy,
        VaultConfigUpdate::AddOrUpdate {
            config: VaultConfigUnchecked {
                addr: "xyz".to_string(),
                deposit_cap: Default::default(),
                max_loan_to_value: Default::default(),
                liquidation_threshold: Default::default(),
                whitelisted: false,
                hls: None,
            },
        },
    );
    assert_err(res, Owner(OwnerError::NotOwner {}));
}

#[test]
fn initializing_asset_param() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.query_owner();
    let vault0 = "vault_addr_0".to_string();
    let vault1 = "vault_addr_1".to_string();

    let starting_vault_config = default_vault_config(&vault0);
    mock.update_vault_config(
        &owner,
        VaultConfigUpdate::AddOrUpdate {
            config: starting_vault_config.clone(),
        },
    )
    .unwrap();

    let all_vault_configs = mock.query_all_vault_configs(None, None);
    assert_eq!(1, all_vault_configs.len());

    // Validate config set correctly
    let config = all_vault_configs.first().unwrap();
    assert_eq!(starting_vault_config, config.clone().into());

    mock.update_vault_config(
        &owner,
        VaultConfigUpdate::AddOrUpdate {
            config: default_vault_config(&vault1),
        },
    )
    .unwrap();

    let vault_configs = mock.query_all_vault_configs(None, None);
    assert_eq!(2, vault_configs.len());
    assert_eq!(&vault1, &vault_configs.get(1).unwrap().addr);
}

#[test]
fn add_same_vault_multiple_times() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.query_owner();
    let vault0 = "vault_addr_0".to_string();

    mock.update_vault_config(
        &owner,
        VaultConfigUpdate::AddOrUpdate {
            config: default_vault_config(&vault0),
        },
    )
    .unwrap();
    mock.update_vault_config(
        &owner,
        VaultConfigUpdate::AddOrUpdate {
            config: default_vault_config(&vault0),
        },
    )
    .unwrap();
    mock.update_vault_config(
        &owner,
        VaultConfigUpdate::AddOrUpdate {
            config: default_vault_config(&vault0),
        },
    )
    .unwrap();
    mock.update_vault_config(
        &owner,
        VaultConfigUpdate::AddOrUpdate {
            config: default_vault_config(&vault0),
        },
    )
    .unwrap();

    let vault_configs = mock.query_all_vault_configs(None, None);
    assert_eq!(1, vault_configs.len());
    assert_eq!(vault0, vault_configs.first().unwrap().addr);
}

#[test]
fn update_existing_vault_configs() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.query_owner();
    let vault0 = "vault_addr_0".to_string();

    let mut config = default_vault_config(&vault0);

    mock.update_vault_config(
        &owner,
        VaultConfigUpdate::AddOrUpdate {
            config: config.clone(),
        },
    )
    .unwrap();

    let vault_config = mock.query_vault_config(&vault0);
    assert!(vault_config.whitelisted);
    assert_eq!(vault_config.max_loan_to_value, Decimal::from_str("0.47").unwrap());

    let new_max_ltv = Decimal::from_str("0.39").unwrap();
    config.whitelisted = false;
    config.max_loan_to_value = new_max_ltv;

    mock.update_vault_config(
        &owner,
        VaultConfigUpdate::AddOrUpdate {
            config,
        },
    )
    .unwrap();

    let all_vault_configs = mock.query_all_vault_configs(None, None);
    assert_eq!(1, all_vault_configs.len());

    let vault_config = mock.query_vault_config(&vault0);
    assert!(!vault_config.whitelisted);
    assert_eq!(vault_config.max_loan_to_value, new_max_ltv);
}

#[test]
fn pagination_query() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.query_owner();
    let vault0 = "vault_addr_0".to_string();
    let vault1 = "vault_addr_1".to_string();
    let vault2 = "vault_addr_2".to_string();
    let vault3 = "vault_addr_3".to_string();
    let vault4 = "vault_addr_4".to_string();
    let vault5 = "vault_addr_5".to_string();

    mock.update_vault_config(
        &owner,
        VaultConfigUpdate::AddOrUpdate {
            config: default_vault_config(&vault0),
        },
    )
    .unwrap();
    mock.update_vault_config(
        &owner,
        VaultConfigUpdate::AddOrUpdate {
            config: default_vault_config(&vault1),
        },
    )
    .unwrap();
    mock.update_vault_config(
        &owner,
        VaultConfigUpdate::AddOrUpdate {
            config: default_vault_config(&vault2),
        },
    )
    .unwrap();
    mock.update_vault_config(
        &owner,
        VaultConfigUpdate::AddOrUpdate {
            config: default_vault_config(&vault3),
        },
    )
    .unwrap();
    mock.update_vault_config(
        &owner,
        VaultConfigUpdate::AddOrUpdate {
            config: default_vault_config(&vault4),
        },
    )
    .unwrap();
    mock.update_vault_config(
        &owner,
        VaultConfigUpdate::AddOrUpdate {
            config: default_vault_config(&vault5),
        },
    )
    .unwrap();

    let vault_configs_a = mock.query_all_vault_configs(None, Some(2));
    let vault_configs_b =
        mock.query_all_vault_configs(vault_configs_a.last().map(|r| r.addr.to_string()), Some(2));
    let vault_configs_c =
        mock.query_all_vault_configs(vault_configs_b.last().map(|r| r.addr.to_string()), None);

    let combined = vault_configs_a
        .iter()
        .cloned()
        .chain(vault_configs_b.iter().cloned())
        .chain(vault_configs_c.iter().cloned())
        .map(|r| r.addr.to_string())
        .collect::<Vec<_>>();

    assert_eq!(6, combined.len());

    assert_contents_equal(&[vault0, vault1, vault2, vault3, vault4, vault5], &combined)
}
