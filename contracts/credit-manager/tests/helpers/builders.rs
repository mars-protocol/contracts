use cosmwasm_std::{coin, Decimal};

use rover::traits::IntoDecimal;

use crate::helpers::{CoinInfo, VaultTestInfo};

pub fn build_mock_coin_infos(count: usize) -> Vec<CoinInfo> {
    (1..=count)
        .into_iter()
        .map(|i| CoinInfo {
            denom: format!("coin_{}", i),
            max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
            price: 10.to_dec().unwrap(),
        })
        .collect()
}

pub fn build_mock_vaults(count: usize) -> Vec<VaultTestInfo> {
    (1..=count)
        .into_iter()
        .map(|i| {
            VaultTestInfo {
                denom: format!("vault_{}", i),
                lockup: Some(1_209_600), // 14 days
                underlying_denoms: vec!["uatom".to_string(), "uosmo".to_string()],
                deposit_cap: coin(10000000, "uusdc"),
            }
        })
        .collect()
}
