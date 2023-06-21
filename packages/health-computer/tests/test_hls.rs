use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{coin, Addr, Coin, Decimal, Uint128};
use mars_params::types::{hls::HlsParams, vault::VaultConfig};
use mars_rover::{
    adapters::vault::{
        CoinValue, Vault, VaultAmount, VaultPosition, VaultPositionAmount, VaultPositionValue,
    },
    msg::query::{DebtAmount, Positions},
};
use mars_rover_health_computer::{DenomsData, HealthComputer, VaultsData};
use mars_rover_health_types::AccountKind;

use crate::helpers::{udai_info, ustars_info};

pub mod helpers;

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
            deposits: vec![Coin {
                denom: ustars.denom.clone(),
                amount: deposit_amount,
            }],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
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
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (ustars.denom.clone(), ustars.price),
            (udai.denom.clone(), udai.price),
        ]),
        params: HashMap::from([
            (ustars.denom.clone(), ustars.params.clone()),
            (udai.denom.clone(), udai.params.clone()),
        ]),
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
            deposits: vec![coin(1200, &ustars.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.denom,
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: ustars.denom,
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::new(5264))),
            }],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(6318574763758));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(4738931072818));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(5054859811006));
    assert_eq!(health.total_debt_value, Uint128::new(1053095794055));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("4.499999999592154861").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("4.799999999565091796").unwrap())
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
            deposits: vec![Coin {
                denom: ustars.denom.clone(),
                amount: deposit_amount,
            }],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
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
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (ustars.denom.clone(), ustars.price),
            (udai.denom.clone(), udai.price),
        ]),
        params: HashMap::from([
            (ustars.denom.clone(), ustars.params.clone()),
            (udai.denom.clone(), udai.params.clone()),
        ]),
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
            deposits: vec![coin(1200, &ustars.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.denom,
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: ustars.denom,
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::new(5264))),
            }],
        },
        denoms_data,
        vaults_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(6318574763758));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(4738931068870));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(5054859811006));
    assert_eq!(health.total_debt_value, Uint128::new(1053095794055));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("4.499999995843208163").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("4.799999999565091796").unwrap())
    );
    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}
