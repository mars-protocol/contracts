use std::collections::HashMap;

use cosmwasm_std::{coin, Addr, Uint128};
use mars_rover::{
    adapters::vault::{
        CoinValue, Vault, VaultAmount, VaultConfig, VaultPosition, VaultPositionAmount,
        VaultPositionValue,
    },
    msg::query::{DebtAmount, Positions},
};
use mars_rover_health_computer::{DenomsData, HealthComputer, VaultsData};
use mars_rover_health_types::HealthError;

use crate::helpers::{udai_info, umars_info};

pub mod helpers;

#[test]
fn missing_price_data() {
    let umars = umars_info();
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([(umars.market.denom.clone(), umars.price)]),
        markets: HashMap::from([
            (umars.market.denom.clone(), umars.market.clone()),
            (udai.market.denom.clone(), udai.market.clone()),
        ]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let h = HealthComputer {
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.market.denom), coin(33, &udai.market.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.market.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.market.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
        allowed_coins: vec![umars.market.denom, udai.market.denom.clone()],
    };

    let err: HealthError = h.compute_health().unwrap_err();
    assert_eq!(err, HealthError::MissingPrice(udai.market.denom))
}

#[test]
fn missing_market_data() {
    let umars = umars_info();
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.market.denom.clone(), umars.price),
            (udai.market.denom.clone(), udai.price),
        ]),
        markets: HashMap::from([(udai.market.denom.clone(), udai.market.clone())]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let h = HealthComputer {
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.market.denom), coin(33, &udai.market.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.market.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.market.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
        allowed_coins: vec![umars.market.denom.clone(), udai.market.denom],
    };

    let err: HealthError = h.compute_health().unwrap_err();
    assert_eq!(err, HealthError::MissingMarket(umars.market.denom))
}

#[test]
fn missing_market_data_for_vault_base_token() {
    let denoms_data = DenomsData {
        prices: HashMap::default(),
        markets: HashMap::default(),
    };

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "leverage_vault_123".to_string(),
                    amount: Default::default(),
                    value: Default::default(),
                },
                base_coin: CoinValue {
                    denom: "base_token_xyz".to_string(),
                    amount: Default::default(),
                    value: Default::default(),
                },
            },
        )]),
        vault_configs: HashMap::from([(
            vault.address.clone(),
            VaultConfig {
                deposit_cap: Default::default(),
                max_ltv: Default::default(),
                liquidation_threshold: Default::default(),
                whitelisted: false,
            },
        )]),
    };

    let h = HealthComputer {
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![],
            debts: vec![],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::one())),
            }],
        },
        denoms_data,
        vaults_data,
        allowed_coins: vec![],
    };

    let err: HealthError = h.compute_health().unwrap_err();
    assert_eq!(err, HealthError::MissingMarket("base_token_xyz".to_string()))
}

#[test]
fn missing_vault_value() {
    let denoms_data = DenomsData {
        prices: HashMap::default(),
        markets: HashMap::default(),
    };

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: HashMap::from([(
            vault.address.clone(),
            VaultConfig {
                deposit_cap: Default::default(),
                max_ltv: Default::default(),
                liquidation_threshold: Default::default(),
                whitelisted: false,
            },
        )]),
    };

    let h = HealthComputer {
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![],
            debts: vec![],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault: vault.clone(),
                amount: VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::one())),
            }],
        },
        denoms_data,
        vaults_data,
        allowed_coins: vec![],
    };

    let err: HealthError = h.compute_health().unwrap_err();
    assert_eq!(err, HealthError::MissingVaultValues(vault.address.to_string()))
}

#[test]
fn missing_vault_config() {
    let denoms_data = DenomsData {
        prices: HashMap::default(),
        markets: HashMap::default(),
    };

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "abc".to_string(),
                    amount: Default::default(),
                    value: Default::default(),
                },
                base_coin: CoinValue {
                    denom: "xyz".to_string(),
                    amount: Default::default(),
                    value: Default::default(),
                },
            },
        )]),
        vault_configs: HashMap::default(),
    };

    let h = HealthComputer {
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![],
            debts: vec![],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault: vault.clone(),
                amount: VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::one())),
            }],
        },
        denoms_data,
        vaults_data,
        allowed_coins: vec![],
    };

    let err: HealthError = h.compute_health().unwrap_err();
    assert_eq!(err, HealthError::MissingVaultConfig(vault.address.to_string()))
}
