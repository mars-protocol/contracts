use cosmwasm_std::{coin, Decimal};
use cw_utils::Duration;
use mars_params::types::asset::LiquidationBonus;

use crate::helpers::{lp_token_info, CoinInfo, VaultTestInfo};

pub fn build_mock_coin_infos(count: usize) -> Vec<CoinInfo> {
    (1..=count)
        .map(|i| CoinInfo {
            denom: format!("coin_{i}"),
            max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
            price: Decimal::from_atomics(10u128, 0).unwrap(),
            liquidation_bonus: LiquidationBonus {
                starting_lb: Decimal::percent(1u64),
                slope: Decimal::from_atomics(2u128, 0).unwrap(),
                min_lb: Decimal::percent(2u64),
                max_lb: Decimal::percent(10u64),
            },
            protocol_liquidation_fee: Decimal::percent(2u64),
            whitelisted: true,
            hls: None,
        })
        .collect()
}

pub fn build_mock_vaults(count: usize) -> Vec<VaultTestInfo> {
    let lp_token = lp_token_info();
    (1..=count)
        .map(|i| {
            VaultTestInfo {
                vault_token_denom: format!("vault_{i}"),
                lockup: Some(Duration::Time(1_209_600)), // 14 days
                base_token_denom: lp_token.denom.clone(),
                deposit_cap: coin(10000000, "uusdc"),
                max_ltv: lp_token.max_ltv,
                liquidation_threshold: lp_token.liquidation_threshold,
                whitelisted: true,
                hls: None,
            }
        })
        .collect()
}
