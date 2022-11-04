use cosmwasm_std::{coin, Addr, Decimal};

use mars_rover::adapters::swap::SwapperBase;
use mars_rover::adapters::vault::{VaultBase, VaultConfig};
use mars_rover::adapters::{OracleBase, RedBankBase, ZapperBase};
use mars_rover::error::ContractError;
use mars_rover::msg::instantiate::{ConfigUpdates, VaultInstantiateConfig};

use crate::helpers::{assert_err, locked_vault_info, uatom_info, uosmo_info, MockEnv};

pub mod helpers;

#[test]
fn test_only_owner_can_update_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let new_owner = Addr::unchecked("bad_guy");

    let res = mock.update_config(
        &new_owner,
        ConfigUpdates {
            account_nft: None,
            owner: Some(new_owner.to_string()),
            allowed_coins: None,
            red_bank: None,
            oracle: None,
            max_liquidation_bonus: None,
            max_close_factor: None,
            swapper: None,
            vault_configs: None,
            zapper: None,
        },
    );

    if res.is_ok() {
        panic!("only owner should be able to update config");
    }
}

#[test]
fn test_raises_on_invalid_vaults_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();
    let res = mock.update_config(
        &Addr::unchecked(original_config.owner.clone()),
        ConfigUpdates {
            account_nft: None,
            owner: None,
            allowed_coins: None,
            red_bank: None,
            oracle: None,
            max_liquidation_bonus: None,
            max_close_factor: None,
            swapper: None,
            vault_configs: Some(vec![VaultInstantiateConfig {
                vault: VaultBase::new("vault_123".to_string()),
                config: VaultConfig {
                    deposit_cap: coin(10_000_000, "uusdc"),
                    max_ltv: Decimal::from_atomics(8u128, 1).unwrap(),
                    liquidation_threshold: Decimal::from_atomics(7u128, 1).unwrap(),
                    whitelisted: true,
                },
            }]),
            zapper: None,
        },
    );

    assert_err(res, ContractError::InvalidVaultConfig {});

    let res = mock.update_config(
        &Addr::unchecked(original_config.owner),
        ConfigUpdates {
            account_nft: None,
            owner: None,
            allowed_coins: None,
            red_bank: None,
            oracle: None,
            max_liquidation_bonus: None,
            max_close_factor: None,
            swapper: None,
            vault_configs: Some(vec![VaultInstantiateConfig {
                vault: VaultBase::new("vault_123".to_string()),
                config: VaultConfig {
                    deposit_cap: coin(10_000_000, "uusdc"),
                    max_ltv: Decimal::from_atomics(8u128, 1).unwrap(),
                    liquidation_threshold: Decimal::from_atomics(9u128, 0).unwrap(),
                    whitelisted: true,
                },
            }]),
            zapper: None,
        },
    );

    assert_err(res, ContractError::InvalidVaultConfig {});
}

#[test]
fn test_update_config_works_with_full_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();
    let original_allowed_coins = mock.query_allowed_coins(None, None);
    let original_vault_configs = mock.query_vault_configs(None, None);

    let new_nft_contract = mock.deploy_nft_contract().unwrap();
    let new_owner = Addr::unchecked("new_owner");
    let new_red_bank = RedBankBase::new("new_red_bank".to_string());
    let new_vault_configs = vec![VaultInstantiateConfig {
        vault: VaultBase::new("vault_contract_3000".to_string()),
        config: VaultConfig {
            deposit_cap: coin(123, "usomething"),
            max_ltv: Decimal::from_atomics(3u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(5u128, 1).unwrap(),
            whitelisted: false,
        },
    }];
    let new_allowed_coins = vec!["uosmo".to_string()];
    let new_oracle = OracleBase::new("new_oracle".to_string());
    let new_zapper = ZapperBase::new("new_zapper".to_string());
    let new_liq_bonus = Decimal::from_atomics(17u128, 2).unwrap();
    let new_close_factor = Decimal::from_atomics(32u128, 2).unwrap();
    let new_swapper = SwapperBase::new("new_swapper".to_string());

    mock.update_config(
        &Addr::unchecked(original_config.owner.clone()),
        ConfigUpdates {
            account_nft: Some(new_nft_contract.to_string()),
            owner: Some(new_owner.to_string()),
            allowed_coins: Some(new_allowed_coins.clone()),
            red_bank: Some(new_red_bank.clone()),
            oracle: Some(new_oracle.clone()),
            max_liquidation_bonus: Some(new_liq_bonus),
            max_close_factor: Some(new_close_factor),
            swapper: Some(new_swapper.clone()),
            vault_configs: Some(new_vault_configs.clone()),
            zapper: Some(new_zapper.clone()),
        },
    )
    .unwrap();

    let new_config = mock.query_config();
    let new_queried_allowed_coins = mock.query_allowed_coins(None, None);
    let new_queried_vault_configs = mock.query_vault_configs(None, None);

    assert_eq!(new_config.account_nft, Some(new_nft_contract.to_string()));
    assert_ne!(new_config.account_nft, original_config.account_nft);

    assert_eq!(new_config.owner, new_owner.to_string());
    assert_ne!(new_config.owner, original_config.owner);

    assert_eq!(new_queried_vault_configs, new_vault_configs);
    assert_ne!(new_queried_vault_configs, original_vault_configs);

    assert_eq!(new_queried_allowed_coins, new_allowed_coins);
    assert_ne!(new_queried_allowed_coins, original_allowed_coins);

    assert_eq!(&new_config.red_bank, new_red_bank.address());
    assert_ne!(new_config.red_bank, original_config.red_bank);

    assert_eq!(&new_config.oracle, new_oracle.address());
    assert_ne!(new_config.oracle, original_config.oracle);

    assert_eq!(&new_config.zapper, new_zapper.address());
    assert_ne!(new_config.zapper, original_config.zapper);

    assert_eq!(new_config.max_liquidation_bonus, new_liq_bonus);
    assert_ne!(
        new_config.max_liquidation_bonus,
        original_config.max_liquidation_bonus
    );

    assert_eq!(new_config.max_close_factor, new_close_factor);
    assert_ne!(
        new_config.max_close_factor,
        original_config.max_close_factor
    );

    assert_eq!(&new_config.swapper, new_swapper.address());
    assert_ne!(new_config.swapper, original_config.swapper);
}

#[test]
fn test_update_config_works_with_some_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();
    let original_allowed_coins = mock.query_allowed_coins(None, None);
    let original_vault_configs = mock.query_vault_configs(None, None);

    let new_nft_contract = mock.deploy_nft_contract().unwrap();
    let new_vault_configs = vec![VaultInstantiateConfig {
        vault: VaultBase::new("vault_contract_1".to_string()),
        config: VaultConfig {
            deposit_cap: coin(1211, "uxyz"),
            max_ltv: Default::default(),
            liquidation_threshold: Default::default(),
            whitelisted: false,
        },
    }];

    mock.update_config(
        &Addr::unchecked(original_config.owner.clone()),
        ConfigUpdates {
            account_nft: Some(new_nft_contract.to_string()),
            vault_configs: Some(new_vault_configs.clone()),
            ..Default::default()
        },
    )
    .unwrap();

    let new_config = mock.query_config();
    let new_queried_allowed_coins = mock.query_allowed_coins(None, None);
    let new_queried_vault_configs = mock.query_vault_configs(None, None);

    // Changed configs
    assert_eq!(new_config.account_nft, Some(new_nft_contract.to_string()));
    assert_ne!(new_config.account_nft, original_config.account_nft);

    assert_eq!(new_queried_vault_configs, new_vault_configs);
    assert_ne!(new_queried_vault_configs, original_vault_configs);

    // Unchanged configs
    assert_eq!(new_config.owner, original_config.owner);
    assert_eq!(original_allowed_coins, new_queried_allowed_coins);
    assert_eq!(new_config.red_bank, original_config.red_bank);
}

#[test]
fn test_update_config_removes_properly() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();
    let leverage_vault = locked_vault_info();

    let mut mock = MockEnv::new()
        .allowed_coins(&[uatom, uosmo])
        .allowed_vaults(&[leverage_vault])
        .build()
        .unwrap();

    let allowed_coins = mock.query_allowed_coins(None, None);
    let vault_configs = mock.query_vault_configs(None, None);

    assert_eq!(allowed_coins.len(), 2);
    assert_eq!(vault_configs.len(), 1);

    mock.update_config(
        &Addr::unchecked(mock.query_config().owner),
        ConfigUpdates {
            allowed_coins: Some(vec![]),
            vault_configs: Some(vec![]),
            ..Default::default()
        },
    )
    .unwrap();

    let allowed_coins = mock.query_allowed_coins(None, None);
    let vault_configs = mock.query_vault_configs(None, None);

    // All allowed vaults and coins removed
    assert_eq!(allowed_coins.len(), 0);
    assert_eq!(vault_configs.len(), 0);
}

#[test]
fn test_update_config_does_nothing_when_nothing_is_passed() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();
    let original_vault_configs = mock.query_vault_configs(None, None);
    let original_allowed_coins = mock.query_allowed_coins(None, None);

    mock.update_config(
        &Addr::unchecked(original_config.owner.clone()),
        Default::default(),
    )
    .unwrap();

    let new_config = mock.query_config();
    let new_queried_vault_configs = mock.query_vault_configs(None, None);
    let new_queried_allowed_coins = mock.query_allowed_coins(None, None);

    assert_eq!(new_config.account_nft, original_config.account_nft);
    assert_eq!(new_config.owner, original_config.owner);
    assert_eq!(new_queried_vault_configs, original_vault_configs);
    assert_eq!(new_queried_allowed_coins, original_allowed_coins);
    assert_eq!(new_config.red_bank, original_config.red_bank);
    assert_eq!(new_config.oracle, original_config.oracle);
    assert_eq!(new_config.zapper, original_config.zapper);
    assert_eq!(
        new_config.max_liquidation_bonus,
        original_config.max_liquidation_bonus
    );
    assert_eq!(
        new_config.max_close_factor,
        original_config.max_close_factor
    );
    assert_eq!(new_config.swapper, original_config.swapper);
}
