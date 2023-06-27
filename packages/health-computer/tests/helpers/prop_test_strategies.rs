use std::collections::HashMap;

use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use mars_params::types::{
    asset::{AssetParams, CmSettings, LiquidationBonus, RedBankSettings},
    hls::HlsParams,
    vault::VaultConfig,
};
use mars_rover::{
    adapters::vault::{
        CoinValue, LockingVaultAmount, UnlockingPositions, Vault, VaultAmount, VaultPosition,
        VaultPositionAmount, VaultPositionValue,
    },
    msg::query::{DebtAmount, LentAmount, Positions},
};
use mars_rover_health_computer::{DenomsData, HealthComputer, VaultsData};
use mars_rover_health_types::AccountKind;
use proptest::{
    collection::vec,
    prelude::{Just, Strategy},
    prop_oneof,
};

fn random_account_kind() -> impl Strategy<Value = AccountKind> {
    prop_oneof![Just(AccountKind::Default), Just(AccountKind::HighLeveredStrategy)]
}

fn random_denom() -> impl Strategy<Value = String> {
    (5..=20)
        .prop_flat_map(|len| proptest::string::string_regex(&format!("[a-z]{{{},}}", len)).unwrap())
}

fn random_bool() -> impl Strategy<Value = bool> {
    proptest::bool::ANY
}

fn random_price() -> impl Strategy<Value = Decimal> {
    (1..=10000, 1..6)
        .prop_map(|(price, offset)| Decimal::from_atomics(price as u128, offset as u32).unwrap())
}

fn random_coin_info() -> impl Strategy<Value = AssetParams> {
    (random_denom(), 30..70, 2..10, 80..90, random_bool()).prop_map(
        |(denom, max_ltv, liq_thresh_buffer, hls_base, whitelisted)| {
            let max_loan_to_value = Decimal::from_atomics(max_ltv as u128, 2).unwrap();
            let liquidation_threshold =
                max_loan_to_value + Decimal::from_atomics(liq_thresh_buffer as u128, 2).unwrap();
            let hls_max_ltv = Decimal::from_atomics(hls_base as u128, 2).unwrap();
            let hls_liq_threshold =
                hls_max_ltv + Decimal::from_atomics(liq_thresh_buffer as u128, 2).unwrap();

            AssetParams {
                denom,
                credit_manager: CmSettings {
                    whitelisted,
                    hls: Some(HlsParams {
                        max_loan_to_value: hls_max_ltv,
                        liquidation_threshold: hls_liq_threshold,
                        correlations: vec![],
                    }),
                },
                red_bank: RedBankSettings {
                    deposit_enabled: true,
                    borrow_enabled: true,
                    deposit_cap: Default::default(),
                },
                max_loan_to_value,
                liquidation_threshold,
                liquidation_bonus: LiquidationBonus {
                    starting_lb: Default::default(),
                    slope: Default::default(),
                    min_lb: Default::default(),
                    max_lb: Default::default(),
                },
                protocol_liquidation_fee: Default::default(),
            }
        },
    )
}

fn random_denoms_data() -> impl Strategy<Value = DenomsData> {
    vec((random_coin_info(), random_price()), 1..=5).prop_map(|info| {
        let mut prices = HashMap::new();
        let mut params = HashMap::new();

        for (coin_info, price) in info {
            prices.insert(coin_info.denom.clone(), price);
            params.insert(coin_info.denom.clone(), coin_info);
        }

        DenomsData {
            prices,
            params,
        }
    })
}

fn random_address() -> impl Strategy<Value = String> {
    proptest::string::string_regex("cosmos1[a-zA-Z0-9]{38}").unwrap()
}

fn random_vault_denom() -> impl Strategy<Value = String> {
    (random_denom()).prop_map(|denom| format!("vault_{denom}"))
}

fn random_vault(
    denoms_data: DenomsData,
) -> impl Strategy<Value = (String, VaultPositionValue, VaultConfig)> {
    (
        random_address(),
        random_vault_denom(),
        20..10_000,
        0..1000,
        30..70,
        2..10,
        80..90,
        random_bool(),
    )
        .prop_map(
            move |(
                addr,
                vault_denom,
                vault_val,
                base_val,
                max_ltv,
                liq_thresh_buffer,
                hls_base,
                whitelisted,
            )| {
                let denoms = denoms_data
                    .params
                    .values()
                    .map(|params| params.denom.clone())
                    .collect::<Vec<_>>();
                let base_denom = denoms.first().unwrap();
                let position_val = VaultPositionValue {
                    vault_coin: CoinValue {
                        denom: vault_denom,
                        amount: Default::default(),
                        value: Uint128::new(vault_val as u128),
                    },
                    // The base coin denom should only be from a denom generated from random_denoms_data()
                    base_coin: CoinValue {
                        denom: base_denom.clone(),
                        amount: Default::default(),
                        value: Uint128::new(base_val as u128),
                    },
                };
                let max_loan_to_value = Decimal::from_atomics(max_ltv as u128, 2).unwrap();
                let liquidation_threshold = max_loan_to_value
                    + Decimal::from_atomics(liq_thresh_buffer as u128, 2).unwrap();
                let hls_max_ltv = Decimal::from_atomics(hls_base as u128, 2).unwrap();
                let hls_liq_threshold =
                    hls_max_ltv + Decimal::from_atomics(liq_thresh_buffer as u128, 2).unwrap();

                let config = VaultConfig {
                    addr: Addr::unchecked(addr.clone()),
                    deposit_cap: Default::default(),
                    max_loan_to_value,
                    liquidation_threshold,
                    whitelisted,
                    hls: Some(HlsParams {
                        max_loan_to_value: hls_max_ltv,
                        liquidation_threshold: hls_liq_threshold,
                        correlations: vec![],
                    }),
                };
                (addr, position_val, config)
            },
        )
}

fn random_param_maps() -> impl Strategy<Value = (DenomsData, VaultsData)> {
    random_denoms_data().prop_flat_map(|denoms_data| {
        vec(random_vault(denoms_data.clone()), 0..=3).prop_map(move |vaults| {
            let mut vault_values = HashMap::new();
            let mut vault_configs = HashMap::new();

            for (addr, position_val, config) in vaults {
                let addr = Addr::unchecked(addr.clone());
                vault_values.insert(addr.clone(), position_val);
                vault_configs.insert(addr, config);
            }

            (
                denoms_data.clone(),
                VaultsData {
                    vault_values,
                    vault_configs,
                },
            )
        })
    })
}

fn random_deposits(denoms_data: DenomsData) -> impl Strategy<Value = Vec<Coin>> {
    let denoms = denoms_data.params.keys().cloned().collect::<Vec<String>>();
    let denoms_len = denoms.len();
    vec(
        (0..denoms_len, 1..=10000).prop_map(move |(index, amount)| {
            let denom = denoms.get(index).unwrap().clone();
            let amount = Uint128::new(amount as u128);

            Coin {
                denom,
                amount,
            }
        }),
        0..denoms_len,
    )
}

fn random_debts(denoms_data: DenomsData) -> impl Strategy<Value = Vec<DebtAmount>> {
    let denoms = denoms_data.params.keys().cloned().collect::<Vec<String>>();
    let denoms_len = denoms.len();
    vec(
        (0..denoms_len, 1..=10000).prop_map(move |(index, amount)| {
            let denom = denoms.get(index).unwrap().clone();
            let amount = Uint128::new(amount as u128);

            DebtAmount {
                denom,
                shares: amount * Uint128::new(10),
                amount,
            }
        }),
        0..denoms_len,
    )
}

fn random_lends(denoms_data: DenomsData) -> impl Strategy<Value = Vec<LentAmount>> {
    let denoms = denoms_data.params.keys().cloned().collect::<Vec<String>>();
    let denoms_len = denoms.len();
    vec(
        (0..denoms_len, 1..=10000).prop_map(move |(index, amount)| {
            let denom = denoms.get(index).unwrap().clone();
            let amount = Uint128::new(amount as u128);

            LentAmount {
                denom,
                shares: amount * Uint128::new(10),
                amount,
            }
        }),
        0..denoms_len,
    )
}

fn random_vault_pos_amount() -> impl Strategy<Value = VaultPositionAmount> {
    prop_oneof![
        random_vault_amount().prop_map(VaultPositionAmount::Unlocked),
        random_locking_vault_amount().prop_map(VaultPositionAmount::Locking),
    ]
}

fn random_vault_amount() -> impl Strategy<Value = VaultAmount> {
    (10..=100000).prop_map(|amount| VaultAmount::new(Uint128::new(amount as u128)))
}

fn random_locking_vault_amount() -> impl Strategy<Value = LockingVaultAmount> {
    (random_vault_amount()).prop_map(|locked| LockingVaultAmount {
        locked,
        unlocking: UnlockingPositions::new(vec![]),
    })
}

fn random_vault_positions(vd: VaultsData) -> impl Strategy<Value = Vec<VaultPosition>> {
    let vault_addrs = vd.vault_configs.keys().cloned().collect::<Vec<Addr>>();
    let addrs_len = vault_addrs.len();

    vec(
        (0..addrs_len, random_vault_pos_amount()).prop_map(move |(index, amount)| {
            let addr = vault_addrs.get(index).unwrap().clone();

            VaultPosition {
                vault: Vault::new(addr),
                amount,
            }
        }),
        addrs_len,
    )
}

pub fn random_health_computer() -> impl Strategy<Value = HealthComputer> {
    (random_param_maps()).prop_flat_map(|(denoms_data, vaults_data)| {
        (
            random_account_kind(),
            random_deposits(denoms_data.clone()),
            random_debts(denoms_data.clone()),
            random_lends(denoms_data.clone()),
            random_vault_positions(vaults_data.clone()),
        )
            .prop_map(move |(kind, deposits, debts, lends, vaults)| HealthComputer {
                kind,
                positions: Positions {
                    account_id: "123".to_string(),
                    deposits,
                    debts,
                    lends,
                    vaults,
                },
                denoms_data: denoms_data.clone(),
                vaults_data: vaults_data.clone(),
            })
    })
}
