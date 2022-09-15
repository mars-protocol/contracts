use rover::msg::query::CoinValue;

pub fn get_coin(denom: &str, coins: &[CoinValue]) -> CoinValue {
    coins.iter().find(|cv| cv.denom == denom).unwrap().clone()
}
