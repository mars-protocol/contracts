use crate::helpers::CoinInfo;
use cosmwasm_std::{Coin, Decimal, Uint128};

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

pub trait CoinCreator {
    fn to_coins(&self, amount: u128) -> Vec<Coin>;
}

impl CoinCreator for Vec<CoinInfo> {
    fn to_coins(&self, amount: u128) -> Vec<Coin> {
        self.iter()
            .map(|info| Coin {
                denom: info.denom.clone(),
                amount: Uint128::from(amount),
            })
            .collect()
    }
}
