use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_rover_health_computer::{DenomsData, HealthComputer, VaultsData};
use mars_types::{
    adapters::vault::{
        CoinValue, Vault, VaultAmount, VaultPosition, VaultPositionAmount, VaultPositionValue,
    },
    credit_manager::{DebtAmount, Positions},
    health::{AccountKind, HealthError, SwapKind},
    params::{HlsParams, VaultConfig},
};

use super::helpers::{uatom_info, udai_info, umars_info, ustars_info};

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

    let err: HealthError = h
        .max_swap_amount_estimate(&udai.denom, &umars.denom, &SwapKind::Default, Decimal::zero())
        .unwrap_err();
    assert_eq!(err, HealthError::MissingPrice(udai.denom));
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
                    denom: udai.denom.clone(),
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

    let res = h
        .max_swap_amount_estimate(&umars.denom, &udai.denom, &SwapKind::Default, Decimal::zero())
        .unwrap();
    assert_eq!(res, Uint128::zero());
}

#[test]
fn deposit_not_present() {
    let udai = udai_info();

    let denoms_data = DenomsData {
        prices: Default::default(),
        params: Default::default(),
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

            deposits: vec![],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let max_withdraw_amount = h
        .max_swap_amount_estimate("xyz", &udai.denom, &SwapKind::Default, Decimal::zero())
        .unwrap();
    assert_eq!(max_withdraw_amount, Uint128::zero());
}

#[test]
fn zero_when_unhealthy() {
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

    let health = h.compute_health().unwrap();
    assert!(health.max_ltv_health_factor < Some(Decimal::one()));
    let max_swap_amount = h
        .max_swap_amount_estimate(&udai.denom, &umars.denom, &SwapKind::Default, Decimal::zero())
        .unwrap();
    assert_eq!(Uint128::zero(), max_swap_amount);
}

#[test]
fn no_debts() {
    let ustars = ustars_info();
    let umars = umars_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (ustars.denom.clone(), ustars.price),
            (umars.denom.clone(), umars.price),
        ]),
        params: HashMap::from([
            (ustars.denom.clone(), ustars.params.clone()),
            (umars.denom.clone(), umars.params.clone()),
        ]),
    };

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let deposit_amount = Uint128::new(1200);
    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            account_kind: AccountKind::Default,

            deposits: vec![coin(deposit_amount.u128(), &ustars.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
        },
        denoms_data,
        vaults_data,
    };

    let max_swap_amount = h
        .max_swap_amount_estimate(&ustars.denom, &umars.denom, &SwapKind::Default, Decimal::zero())
        .unwrap();
    assert_eq!(deposit_amount, max_swap_amount);
}

#[test]
fn should_allow_max_swap() {
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

    let deposit_amount = Uint128::new(33);
    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            account_kind: AccountKind::Default,

            deposits: vec![coin(1200, &umars.denom), coin(deposit_amount.u128(), &udai.denom)],
            debts: vec![DebtAmount {
                denom: udai.denom.clone(),
                shares: Default::default(),
                amount: Uint128::new(5),
            }],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
        },
        denoms_data,
        vaults_data,
    };

    // Max when debt value is smaller than collateral value - withdraw denom value
    let max_swap_amount = h
        .max_swap_amount_estimate(&udai.denom, &umars.denom, &SwapKind::Default, Decimal::zero())
        .unwrap();
    assert_eq!(deposit_amount, max_swap_amount);
}

#[test]
fn hls_with_max_withdraw() {
    let ustars = ustars_info();
    let uatom = uatom_info();

    let denoms_data = DenomsData {
        prices: HashMap::from([
            (ustars.denom.clone(), ustars.price),
            (uatom.denom.clone(), uatom.price),
        ]),
        params: HashMap::from([
            (ustars.denom.clone(), ustars.params.clone()),
            (uatom.denom.clone(), uatom.params.clone()),
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

    let max_before = h
        .max_swap_amount_estimate(&ustars.denom, &uatom.denom, &SwapKind::Default, Decimal::zero())
        .unwrap();
    h.kind = AccountKind::HighLeveredStrategy;
    let max_after = h
        .max_swap_amount_estimate(&ustars.denom, &uatom.denom, &SwapKind::Default, Decimal::zero())
        .unwrap();
    assert!(max_after > max_before)
}
