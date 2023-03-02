use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use cw_multi_test::{BasicApp, Executor};
use mars_mock_oracle::msg::{CoinPrice, InstantiateMsg as OracleInstantiateMsg};
use mars_mock_vault::msg::InstantiateMsg as VaultInstantiateMsg;
use mars_rover::{
    adapters::{
        health::HealthContractUnchecked,
        oracle::{OracleBase, OracleUnchecked},
        swap::SwapperBase,
        vault::{VaultBase, VaultConfig},
        zapper::ZapperBase,
    },
    error::ContractError::InvalidConfig,
    msg::{
        instantiate::{ConfigUpdates, VaultInstantiateConfig},
        query::VaultConfigResponse,
    },
};

use crate::helpers::{
    assert_err, locked_vault_info, mock_oracle_contract, mock_vault_contract, uatom_info,
    uosmo_info, MockEnv,
};

pub mod helpers;

#[test]
fn only_owner_can_update_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let new_owner = Addr::unchecked("bad_guy");

    let res = mock.update_config(
        &new_owner,
        ConfigUpdates {
            account_nft: None,
            allowed_coins: None,
            oracle: None,
            max_close_factor: None,
            max_unlocking_positions: None,
            swapper: None,
            vault_configs: None,
            zapper: None,
            health_contract: None,
        },
    );

    if res.is_ok() {
        panic!("only owner should be able to update config");
    }
}

#[test]
fn raises_on_invalid_vaults_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();

    let mut vault_config = deploy_vault(&mut mock.app);

    // Invalid config. Max LTV should be lower than liquidation threshold.
    vault_config.config.max_ltv = Decimal::from_atomics(8u128, 1).unwrap();
    vault_config.config.liquidation_threshold = Decimal::from_atomics(7u128, 1).unwrap();

    let res = mock.update_config(
        &Addr::unchecked(original_config.owner.clone().unwrap()),
        ConfigUpdates {
            account_nft: None,
            allowed_coins: None,
            oracle: None,
            max_close_factor: None,
            max_unlocking_positions: None,
            swapper: None,
            vault_configs: Some(vec![vault_config]),
            zapper: None,
            health_contract: None,
        },
    );

    assert_err(
        res,
        InvalidConfig {
            reason: "max ltv or liquidation threshold are invalid".to_string(),
        },
    );

    let mut vault_config = deploy_vault(&mut mock.app);

    // Invalid config. Liquidation threshold should be <= 1.
    vault_config.config.liquidation_threshold = Decimal::from_atomics(9u128, 0).unwrap();

    let res = mock.update_config(
        &Addr::unchecked(original_config.owner.clone().unwrap()),
        ConfigUpdates {
            account_nft: None,
            allowed_coins: None,
            oracle: None,
            max_close_factor: None,
            max_unlocking_positions: None,
            swapper: None,
            vault_configs: Some(vec![vault_config]),
            zapper: None,
            health_contract: None,
        },
    );

    assert_err(
        res,
        InvalidConfig {
            reason: "max ltv or liquidation threshold are invalid".to_string(),
        },
    );

    // Duplicate vault tokens
    let vault_a = deploy_vault(&mut mock.app);
    let vault_b = deploy_vault(&mut mock.app);

    let res = mock.update_config(
        &Addr::unchecked(original_config.owner.unwrap()),
        ConfigUpdates {
            account_nft: None,
            allowed_coins: None,
            oracle: None,
            max_close_factor: None,
            max_unlocking_positions: None,
            swapper: None,
            vault_configs: Some(vec![vault_a, vault_b]),
            zapper: None,
            health_contract: None,
        },
    );

    assert_err(
        res,
        InvalidConfig {
            reason: "Multiple vaults share the same vault token".to_string(),
        },
    );
}

#[test]
fn update_config_works_with_full_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();
    let original_allowed_coins = mock.query_allowed_coins(None, None);
    let original_vault_configs = mock.query_vault_configs(None, None);

    let new_nft_contract = mock.deploy_new_nft_contract().unwrap();
    let new_vault_configs = vec![deploy_vault(&mut mock.app)];
    let new_allowed_coins = vec!["uosmo".to_string()];
    let new_oracle = deploy_new_oracle(&mut mock.app);
    let new_zapper = ZapperBase::new("new_zapper".to_string());
    let new_close_factor = Decimal::from_atomics(32u128, 2).unwrap();
    let new_unlocking_max = Uint128::new(321);
    let new_swapper = SwapperBase::new("new_swapper".to_string());
    let new_health_contract = HealthContractUnchecked::new("new_health_contract".to_string());

    mock.update_config(
        &Addr::unchecked(original_config.owner.clone().unwrap()),
        ConfigUpdates {
            account_nft: Some(new_nft_contract.to_string()),
            allowed_coins: Some(new_allowed_coins.clone()),
            oracle: Some(new_oracle.clone()),
            max_close_factor: Some(new_close_factor),
            max_unlocking_positions: Some(new_unlocking_max),
            swapper: Some(new_swapper.clone()),
            vault_configs: Some(new_vault_configs.clone()),
            zapper: Some(new_zapper.clone()),
            health_contract: Some(new_health_contract.clone()),
        },
    )
    .unwrap();

    let new_config = mock.query_config();
    let new_queried_allowed_coins = mock.query_allowed_coins(None, None);
    let new_queried_vault_configs = mock.query_vault_configs(None, None);

    assert_eq!(new_config.account_nft, Some(new_nft_contract.to_string()));
    assert_ne!(new_config.account_nft, original_config.account_nft);

    assert_eq!(new_config.owner.unwrap(), original_config.owner.clone().unwrap());

    assert_eq!(
        new_queried_vault_configs,
        new_vault_configs
            .iter()
            .map(|v| VaultConfigResponse {
                vault: v.vault.clone(),
                config: v.config.clone(),
            })
            .collect::<Vec<_>>()
    );
    assert_ne!(new_queried_vault_configs, original_vault_configs);

    assert_eq!(new_queried_allowed_coins, new_allowed_coins);
    assert_ne!(new_queried_allowed_coins, original_allowed_coins);

    assert_eq!(&new_config.oracle, new_oracle.address());
    assert_ne!(new_config.oracle, original_config.oracle);

    assert_eq!(&new_config.zapper, new_zapper.address());
    assert_ne!(new_config.zapper, original_config.zapper);

    assert_eq!(new_config.max_close_factor, new_close_factor);
    assert_ne!(new_config.max_close_factor, original_config.max_close_factor);

    assert_eq!(new_config.max_unlocking_positions, new_unlocking_max);
    assert_ne!(new_config.max_unlocking_positions, original_config.max_unlocking_positions);

    assert_eq!(&new_config.swapper, new_swapper.address());
    assert_ne!(new_config.swapper, original_config.swapper);

    assert_eq!(&new_config.health_contract, new_health_contract.address());
    assert_ne!(new_config.health_contract, original_config.health_contract);
}

#[test]
fn update_config_works_with_some_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();
    let original_allowed_coins = mock.query_allowed_coins(None, None);
    let original_vault_configs = mock.query_vault_configs(None, None);

    let new_nft_contract = mock.deploy_new_nft_contract().unwrap();
    let new_max_unlocking = Uint128::new(42);

    mock.update_config(
        &Addr::unchecked(original_config.owner.clone().unwrap()),
        ConfigUpdates {
            account_nft: Some(new_nft_contract.to_string()),
            max_unlocking_positions: Some(new_max_unlocking),
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

    assert_eq!(new_config.max_unlocking_positions, new_max_unlocking);
    assert_ne!(new_config.max_unlocking_positions, original_config.max_unlocking_positions);

    // Unchanged configs
    assert_eq!(new_config.owner, original_config.owner);
    assert_eq!(new_config.proposed_new_owner, original_config.proposed_new_owner);
    assert_eq!(new_config.red_bank, original_config.red_bank);
    assert_eq!(new_config.oracle, original_config.oracle);
    assert_eq!(new_config.max_close_factor, original_config.max_close_factor);
    assert_eq!(new_config.swapper, original_config.swapper);
    assert_eq!(new_config.zapper, original_config.zapper);
    assert_eq!(new_config.health_contract, original_config.health_contract);
    assert_eq!(original_allowed_coins, new_queried_allowed_coins);
    assert_eq!(new_queried_vault_configs, original_vault_configs);
}

#[test]
fn update_config_removes_properly() {
    let uatom = uatom_info();
    let uosmo = uosmo_info();
    let leverage_vault = locked_vault_info();

    let mut mock = MockEnv::new()
        .allowed_coins(&[uatom, uosmo])
        .vault_configs(&[leverage_vault])
        .build()
        .unwrap();

    let allowed_coins = mock.query_allowed_coins(None, None);
    let vault_configs = mock.query_vault_configs(None, None);

    assert_eq!(allowed_coins.len(), 2);
    assert_eq!(vault_configs.len(), 1);

    mock.update_config(
        &Addr::unchecked(mock.query_config().owner.unwrap()),
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
fn update_config_does_nothing_when_nothing_is_passed() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();
    let original_vault_configs = mock.query_vault_configs(None, None);
    let original_allowed_coins = mock.query_allowed_coins(None, None);

    mock.update_config(
        &Addr::unchecked(original_config.owner.clone().unwrap()),
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
    assert_eq!(new_config.max_close_factor, original_config.max_close_factor);
    assert_eq!(new_config.swapper, original_config.swapper);
    assert_eq!(new_config.health_contract, original_config.health_contract);
}

#[test]
fn max_close_factor_validated_on_update() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();
    let res = mock.update_config(
        &Addr::unchecked(original_config.owner.unwrap()),
        ConfigUpdates {
            max_close_factor: Some(Decimal::from_atomics(42u128, 1).unwrap()),
            ..Default::default()
        },
    );

    assert_err(
        res,
        InvalidConfig {
            reason: "value greater than one".to_string(),
        },
    );
}

#[test]
fn raises_on_duplicate_vault_configs() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();
    let res = mock.update_config(
        &Addr::unchecked(original_config.owner.unwrap()),
        ConfigUpdates {
            account_nft: None,
            allowed_coins: None,
            oracle: None,
            max_close_factor: None,
            max_unlocking_positions: None,
            swapper: None,
            vault_configs: Some(vec![
                VaultInstantiateConfig {
                    vault: VaultBase::new("vault_123".to_string()),
                    config: VaultConfig {
                        deposit_cap: Default::default(),
                        max_ltv: Default::default(),
                        liquidation_threshold: Default::default(),
                        whitelisted: true,
                    },
                },
                VaultInstantiateConfig {
                    vault: VaultBase::new("vault_123".to_string()),
                    config: VaultConfig {
                        deposit_cap: Default::default(),
                        max_ltv: Default::default(),
                        liquidation_threshold: Default::default(),
                        whitelisted: false,
                    },
                },
            ]),
            zapper: None,
            health_contract: None,
        },
    );

    assert_err(
        res,
        InvalidConfig {
            reason: "Duplicate vault configs present".to_string(),
        },
    );
}

#[test]
fn raises_on_duplicate_coin_configs() {
    let mut mock = MockEnv::new().build().unwrap();
    let original_config = mock.query_config();
    let res = mock.update_config(
        &Addr::unchecked(original_config.owner.unwrap()),
        ConfigUpdates {
            account_nft: None,
            allowed_coins: Some(vec![
                "uosmo".to_string(),
                "uatom".to_string(),
                "uosmo".to_string(),
            ]),
            oracle: None,
            max_close_factor: None,
            max_unlocking_positions: None,
            swapper: None,
            vault_configs: None,
            zapper: None,
            health_contract: None,
        },
    );

    assert_err(
        res,
        InvalidConfig {
            reason: "Duplicate coin configs present".to_string(),
        },
    );
}

fn deploy_new_oracle(app: &mut BasicApp) -> OracleUnchecked {
    let contract_code_id = app.store_code(mock_oracle_contract());
    let addr = app
        .instantiate_contract(
            contract_code_id,
            Addr::unchecked("oracle_contract_owner"),
            &OracleInstantiateMsg {
                prices: vec![
                    CoinPrice {
                        denom: "uusdc".to_string(),
                        price: Decimal::from_atomics(12345u128, 4).unwrap(),
                    },
                    CoinPrice {
                        denom: "vault_xyz".to_string(),
                        price: Decimal::from_atomics(989685877u128, 8).unwrap(),
                    },
                ],
            },
            &[],
            "mock-oracle",
            None,
        )
        .unwrap();
    OracleUnchecked::new(addr.to_string())
}

fn deploy_vault(app: &mut BasicApp) -> VaultInstantiateConfig {
    let code_id = app.store_code(mock_vault_contract());
    let addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked("vault-instantiator"),
            &VaultInstantiateMsg {
                vault_token_denom: "vault_xyz".to_string(),
                lockup: None,
                base_token_denom: "uusdc".to_string(),
                oracle: OracleBase::new("oracle".to_string()),
            },
            &[],
            "mock-vault",
            None,
        )
        .unwrap();
    VaultInstantiateConfig {
        vault: VaultBase::new(addr.to_string()),
        config: VaultConfig {
            deposit_cap: coin(123, "uusdc"),
            max_ltv: Decimal::from_atomics(3u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(5u128, 1).unwrap(),
            whitelisted: false,
        },
    }
}
