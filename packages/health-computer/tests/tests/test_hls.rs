use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{coin, Addr, Coin, Decimal, Uint128};
use mars_rover_health_computer::{DenomsData, HealthComputer, VaultsData};
use mars_types::{
    adapters::vault::{
        CoinValue, Vault, VaultAmount, VaultPosition, VaultPositionAmount, VaultPositionValue,
    },
    credit_manager::{DebtAmount, Positions},
    health::AccountKind,
    params::{HlsParams, VaultConfig},
};

use super::helpers::ustars_info;

#[test]
fn hls_deposit() {
    let ustars = ustars_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([(ustars.denom.clone(), ustars.price)]),
        params: HashMap::from([(ustars.denom.clone(), ustars.params.clone())]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let deposit_amount = Uint128::new(300);
    let h = HealthComputer {
        kind: AccountKind::HighLeveredStrategy,
        positions: Positions {
            account_id: "123".to_string(),
            account_kind: AccountKind::HighLeveredStrategy,

            deposits: vec![Coin {
                denom: ustars.denom.clone(),
                amount: deposit_amount,
            }],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();
    let collateral_value = deposit_amount.checked_mul_floor(ustars.price).unwrap();
    assert_eq!(health.total_collateral_value, collateral_value);
    assert_eq!(
        health.max_ltv_adjusted_collateral,
        collateral_value
            .checked_mul_floor(ustars.params.credit_manager.hls.as_ref().unwrap().max_loan_to_value)
            .unwrap()
    );
    assert_eq!(
        health.liquidation_threshold_adjusted_collateral,
        collateral_value
            .checked_mul_floor(ustars.params.credit_manager.hls.unwrap().liquidation_threshold)
            .unwrap()
    );
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.liquidation_health_factor, None);
    assert_eq!(health.max_ltv_health_factor, None);
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());
}

#[test]
fn hls_vault() {
    let ustars = ustars_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([(ustars.denom.clone(), ustars.price)]),
        params: HashMap::from([(ustars.denom.clone(), ustars.params.clone())]),
    };

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "leverage_vault_123".to_string(),
                    amount: Uint128::new(5264),
                    value: Uint128::new(5264),
                },
                base_coin: CoinValue {
                    denom: ustars.denom.clone(),
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
                max_loan_to_value: Decimal::from_str("0.4").unwrap(),
                liquidation_threshold: Decimal::from_str("0.5").unwrap(),
                whitelisted: true,
                hls: Some(HlsParams {
                    max_loan_to_value: Decimal::from_str("0.75").unwrap(),
                    liquidation_threshold: Decimal::from_str("0.8").unwrap(),
                    correlations: vec![],
                }),
            },
        )]),
    };

    let h = HealthComputer {
        kind: AccountKind::HighLeveredStrategy,
        positions: Positions {
            account_id: "123".to_string(),
            account_kind: AccountKind::HighLeveredStrategy,
            deposits: vec![coin(1200, &ustars.denom)],
            debts: vec![DebtAmount {
                denom: ustars.denom,
                shares: Default::default(),
                amount: Uint128::new(200),
            }],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::new(5264))),
            }],
            staked_astro_lps: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(6318574763758));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(4738931072818));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(5054859811006));
    assert_eq!(health.total_debt_value, Uint128::new(1053095793083));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("4.500000003745623167").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("4.800000003995457989").unwrap())
    );
    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

#[test]
fn hls_on_blacklisted_asset() {
    let mut ustars = ustars_info();
    ustars.params.credit_manager.whitelisted = false;

    let denoms_data = DenomsData {
        prices: HashMap::from([(ustars.denom.clone(), ustars.price)]),
        params: HashMap::from([(ustars.denom.clone(), ustars.params.clone())]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let deposit_amount = Uint128::new(300);
    let h = HealthComputer {
        kind: AccountKind::HighLeveredStrategy,
        positions: Positions {
            account_id: "123".to_string(),
            account_kind: AccountKind::HighLeveredStrategy,
            deposits: vec![Coin {
                denom: ustars.denom.clone(),
                amount: deposit_amount,
            }],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();
    let collateral_value = deposit_amount.checked_mul_floor(ustars.price).unwrap();
    assert_eq!(health.total_collateral_value, collateral_value);
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::zero());
    assert_eq!(
        health.liquidation_threshold_adjusted_collateral,
        collateral_value
            .checked_mul_floor(ustars.params.credit_manager.hls.unwrap().liquidation_threshold)
            .unwrap()
    );
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.liquidation_health_factor, None);
    assert_eq!(health.max_ltv_health_factor, None);
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());
}

#[test]
fn hls_on_blacklisted_vault() {
    let ustars = ustars_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([(ustars.denom.clone(), ustars.price)]),
        params: HashMap::from([(ustars.denom.clone(), ustars.params.clone())]),
    };

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "leverage_vault_123".to_string(),
                    amount: Uint128::new(5264),
                    value: Uint128::new(5264),
                },
                base_coin: CoinValue {
                    denom: ustars.denom.clone(),
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
                max_loan_to_value: Decimal::from_str("0.4").unwrap(),
                liquidation_threshold: Decimal::from_str("0.5").unwrap(),
                whitelisted: false,
                hls: Some(HlsParams {
                    max_loan_to_value: Decimal::from_str("0.75").unwrap(),
                    liquidation_threshold: Decimal::from_str("0.8").unwrap(),
                    correlations: vec![],
                }),
            },
        )]),
    };

    let h = HealthComputer {
        kind: AccountKind::HighLeveredStrategy,
        positions: Positions {
            account_id: "123".to_string(),
            account_kind: AccountKind::HighLeveredStrategy,
            deposits: vec![coin(1200, &ustars.denom)],
            debts: vec![DebtAmount {
                denom: ustars.denom,
                shares: Default::default(),
                amount: Uint128::new(200),
            }],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::new(5264))),
            }],
            staked_astro_lps: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(6318574763758));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(4738931068870));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(5054859811006));
    assert_eq!(health.total_debt_value, Uint128::new(1053095793083));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("4.499999999996676465").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("4.800000003995457989").unwrap())
    );
    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}
