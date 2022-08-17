use cosmwasm_std::Decimal;

use crate::helpers::CoinInfo;

pub fn build_mock_coin_infos(count: usize) -> Vec<CoinInfo> {
    (1..=count)
        .into_iter()
        .map(|i| CoinInfo {
            denom: format!("coin_{}", i),
            max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
            price: Decimal::from_atomics(10u128, 0).unwrap(),
        })
        .collect()
}
