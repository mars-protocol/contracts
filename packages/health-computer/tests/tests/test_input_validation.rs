use std::collections::HashMap;

use cosmwasm_std::{coin, Addr, Uint128};
use mars_rover_health_computer::{DenomsData, HealthComputer, VaultsData};
use mars_types::{
    adapters::vault::{
        CoinValue, Vault, VaultAmount, VaultPosition, VaultPositionAmount, VaultPositionValue,
    },
    credit_manager::{DebtAmount, Positions},
    health::{AccountKind, HealthError},
    params::VaultConfig,
};

use super::helpers::{udai_info, umars_info};

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
            account_kind: AccountKind::Default,

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
            staked_astro_lps: vec![],
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
            account_kind: AccountKind::Default,

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
            staked_astro_lps: vec![],
        },
        denoms_data,
        vaults_data,
    };

    // If asset params is missing for a denom (in params contract), both price and params will be missing in denoms_data.
    // For purpose of this test, we set the price for umars, but not the params.
    let res = h.compute_health().unwrap();
    // mars is not counted in the collateral because it has no params
    let expected_ltv_collateral = Uint128::new(33)
        .checked_mul_floor(udai.price)
        .unwrap()
        .checked_mul_floor(udai.params.max_loan_to_value)
        .unwrap();
    assert_eq!(res.max_ltv_adjusted_collateral, expected_ltv_collateral);
    let expected_liq_collateral = Uint128::new(33)
        .checked_mul_floor(udai.price)
        .unwrap()
        .checked_mul_floor(udai.params.liquidation_threshold)
        .unwrap();
    assert_eq!(res.liquidation_threshold_adjusted_collateral, expected_liq_collateral);
    let udai_debt = Uint128::new(3100).checked_mul_ceil(udai.price).unwrap();
    // mars is counted in the debt because it has a price
    let umars_debt = Uint128::new(200).checked_mul_ceil(umars.price).unwrap();
    let expected_debt = udai_debt + umars_debt;
    assert_eq!(res.total_debt_value, expected_debt);
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
            account_kind: AccountKind::Default,

            deposits: vec![],
            debts: vec![],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::one())),
            }],
            staked_astro_lps: vec![],
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
            account_kind: AccountKind::Default,

            deposits: vec![],
            debts: vec![],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault: vault.clone(),
                amount: VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::one())),
            }],
            staked_astro_lps: vec![],
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
            account_kind: AccountKind::Default,

            deposits: vec![],
            debts: vec![],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault: vault.clone(),
                amount: VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::one())),
            }],
            staked_astro_lps: vec![],
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
            account_kind: AccountKind::HighLeveredStrategy,

            deposits: vec![coin(1200, &umars.denom)],
            debts: vec![DebtAmount {
                denom: umars.denom.clone(),
                shares: Default::default(),
                amount: Uint128::new(200),
            }],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let err: HealthError = h.compute_health().unwrap_err();
    assert_eq!(err, HealthError::MissingHLSParams(umars.denom))
}
