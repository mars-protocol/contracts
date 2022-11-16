use cosmwasm_std::{coin, Decimal};
use cw_utils::Duration;

use mars_rover::traits::IntoDecimal;

use crate::helpers::{lp_token_info, CoinInfo, VaultTestInfo};

pub fn build_mock_coin_infos(count: usize) -> Vec<CoinInfo> {
    (1..=count)
        .into_iter()
        .map(|i| CoinInfo {
            denom: format!("coin_{}", i),
            max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
            price: 10.to_dec().unwrap(),
            liquidation_bonus: Decimal::from_atomics(15u128, 2).unwrap(),
        })
        .collect()
}

pub fn build_mock_vaults(count: usize) -> Vec<VaultTestInfo> {
    let lp_token = lp_token_info();
    (1..=count)
        .into_iter()
        .map(|i| {
            VaultTestInfo {
                vault_token_denom: format!("vault_{}", i),
                lockup: Some(Duration::Time(1_209_600)), // 14 days
                base_token_denom: lp_token.denom.clone(),
                deposit_cap: coin(10000000, "uusdc"),
                max_ltv: lp_token.max_ltv,
                liquidation_threshold: lp_token.liquidation_threshold,
            }
        })
        .collect()
}
