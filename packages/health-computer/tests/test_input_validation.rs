use std::collections::HashMap;

use cosmwasm_std::{coin, Addr, Uint128};
use mars_params::types::vault::VaultConfig;
use mars_rover::{
    adapters::vault::{
        CoinValue, Vault, VaultAmount, VaultPosition, VaultPositionAmount, VaultPositionValue,
    },
    msg::query::{DebtAmount, Positions},
};
use mars_rover_health_computer::{DenomsData, HealthComputer, VaultsData};
use mars_rover_health_types::{AccountKind, HealthError};

use crate::helpers::{udai_info, umars_info};

pub mod helpers;

#[test]
fn missing_price_data() {
    let umars = umars_info();
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([(umars.denom.clone(), umars.price)]),
        params: HashMap::from([
            (umars.denom.clone(), umars.params.clone()),
            (udai.denom.clone(), udai.params.clone()),
        ]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.denom), coin(33, &udai.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.denom,
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let err: HealthError = h.compute_health().unwrap_err();
    assert_eq!(err, HealthError::MissingPrice(udai.denom))
}

#[test]
fn missing_params() {
    let umars = umars_info();
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.denom.clone(), umars.price),
            (udai.denom.clone(), udai.price),
        ]),
        params: HashMap::from([(udai.denom.clone(), udai.params.clone())]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.denom), coin(33, &udai.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.denom,
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let err: HealthError = h.compute_health().unwrap_err();
    assert_eq!(err, HealthError::MissingParams(umars.denom))
}

#[test]
fn missing_market_data_for_vault_base_token() {
    let denoms_data = DenomsData {
        prices: HashMap::default(),
        params: HashMap::default(),
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
                addr: vault.address.clone(),
                deposit_cap: Default::default(),
                max_loan_to_value: Default::default(),
                liquidation_threshold: Default::default(),
                whitelisted: false,
                hls: None,
            },
        )]),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
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
    };

    let err: HealthError = h.compute_health().unwrap_err();
    assert_eq!(err, HealthError::MissingParams("base_token_xyz".to_string()))
}

#[test]
fn missing_vault_value() {
    let denoms_data = DenomsData {
        prices: HashMap::default(),
        params: HashMap::default(),
    };

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: HashMap::from([(
            vault.address.clone(),
            VaultConfig {
                addr: vault.address.clone(),
                deposit_cap: Default::default(),
                max_loan_to_value: Default::default(),
                liquidation_threshold: Default::default(),
                whitelisted: false,
                hls: None,
            },
        )]),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
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
    };

    let err: HealthError = h.compute_health().unwrap_err();
    assert_eq!(err, HealthError::MissingVaultValues(vault.address.to_string()))
}

#[test]
fn missing_vault_config() {
    let denoms_data = DenomsData {
        prices: HashMap::default(),
        params: HashMap::default(),
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
        kind: AccountKind::Default,
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
    };

    let err: HealthError = h.compute_health().unwrap_err();
    assert_eq!(err, HealthError::MissingVaultConfig(vault.address.to_string()))
}

#[test]
fn missing_hls_params() {
    let umars = umars_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([(umars.denom.clone(), umars.price)]),
        params: HashMap::from([(umars.denom.clone(), umars.params.clone())]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::HighLeveredStrategy,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.denom)],
            debts: vec![DebtAmount {
                denom: umars.denom.clone(),
                shares: Default::default(),
                amount: Uint128::new(200),
            }],
            lends: vec![],
            vaults: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let err: HealthError = h.compute_health().unwrap_err();
    assert_eq!(err, HealthError::MissingHLSParams(umars.denom))
}
