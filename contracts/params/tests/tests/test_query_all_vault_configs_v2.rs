use cosmwasm_std::Addr;
use mars_types::params::VaultConfigUpdate;

use crate::tests::helpers::{default_vault_config, MockEnv};

#[test]
fn validate_address_correctly() {
    let vault_config_1 = default_vault_config("vault_1");
    let vault_config_2 = default_vault_config("vault_2");
    let mut mock = MockEnv::new().build().unwrap();

    mock.update_vault_config(
        &mock.query_owner(),
        VaultConfigUpdate::AddOrUpdate {
            config: vault_config_1,
        },
    )
    .unwrap();
    mock.update_vault_config(
        &mock.query_owner(),
        VaultConfigUpdate::AddOrUpdate {
            config: vault_config_2,
        },
    )
    .unwrap();

    let vault_configs = mock.query_all_vault_configs_v2(None, Some(1));

    assert_eq!(vault_configs.data.first().unwrap().addr, Addr::unchecked("vault_1"));
    assert_eq!(vault_configs.data.len(), 1)
}

#[test]
fn allows_setting_limit() {
    let vault_config_1 = default_vault_config("vault_1");
    let vault_config_2 = default_vault_config("vault_2");
    let mut mock = MockEnv::new().build().unwrap();

    mock.update_vault_config(
        &mock.query_owner(),
        VaultConfigUpdate::AddOrUpdate {
            config: vault_config_1,
        },
    )
    .unwrap();
    mock.update_vault_config(
        &mock.query_owner(),
        VaultConfigUpdate::AddOrUpdate {
            config: vault_config_2,
        },
    )
    .unwrap();

    let vault_configs = mock.query_all_vault_configs_v2(None, Some(1));

    assert_eq!(vault_configs.data.first().unwrap().addr, Addr::unchecked("vault_1"));
    assert_eq!(vault_configs.data.len(), 1);
}

#[test]
fn start_after_skips_first() {
    let vault_config_1 = default_vault_config("vault_1");
    let vault_config_2 = default_vault_config("vault_2");
    let mut mock = MockEnv::new().build().unwrap();

    mock.update_vault_config(
        &mock.query_owner(),
        VaultConfigUpdate::AddOrUpdate {
            config: vault_config_1,
        },
    )
    .unwrap();
    mock.update_vault_config(
        &mock.query_owner(),
        VaultConfigUpdate::AddOrUpdate {
            config: vault_config_2,
        },
    )
    .unwrap();

    let vault_configs = mock.query_all_vault_configs_v2(Some("vault_1".to_string()), None);

    assert_eq!(vault_configs.data.len(), 1);
    assert_eq!(vault_configs.data.first().unwrap().addr, Addr::unchecked("vault_2"));
}
