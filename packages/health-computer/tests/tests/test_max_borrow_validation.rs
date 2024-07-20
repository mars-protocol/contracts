use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_rover_health_computer::{DenomsData, HealthComputer, VaultsData};
use mars_types::{
    adapters::vault::{
        CoinValue, Vault, VaultAmount, VaultPosition, VaultPositionAmount, VaultPositionValue,
    },
    credit_manager::{DebtAmount, Positions},
    health::{AccountKind, BorrowTarget, HealthError},
    params::{HlsParams, VaultConfig},
};

use super::helpers::{udai_info, umars_info, ustars_info};

#[test]
fn missing_borrow_denom_price_data() {
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

    let err: HealthError =
        h.max_borrow_amount_estimate(&udai.denom, &BorrowTarget::Deposit).unwrap_err();
    assert_eq!(err, HealthError::MissingPrice(udai.denom));
}

#[test]
fn missing_borrow_denom_params() {
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

    let err: HealthError =
        h.max_borrow_amount_estimate(&umars.denom, &BorrowTarget::Deposit).unwrap_err();
    assert_eq!(err, HealthError::MissingParams(umars.denom));
}

#[test]
fn cannot_borrow_when_unhealthy() {
    let umars = umars_info();
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (umars.denom.clone(), umars.price),
            (udai.denom.clone(), udai.price),
        ]),
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
                    amount: Uint128::new(2500),
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

    let max_withdraw_amount =
        h.max_borrow_amount_estimate(&udai.denom, &BorrowTarget::Deposit).unwrap();
    assert_eq!(Uint128::zero(), max_withdraw_amount);
}

#[test]
fn hls_influences_max_borrow() {
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

    let mut h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            account_kind: AccountKind::Default,

            deposits: vec![coin(1200, &ustars.denom)],
            debts: vec![DebtAmount {
                denom: ustars.denom.clone(),
                shares: Default::default(),
                amount: Uint128::new(800),
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

    let max_before = h.max_borrow_amount_estimate(&ustars.denom, &BorrowTarget::Deposit).unwrap();
    h.kind = AccountKind::HighLeveredStrategy;
    let max_after = h.max_borrow_amount_estimate(&ustars.denom, &BorrowTarget::Deposit).unwrap();
    assert!(max_after > max_before);
}
