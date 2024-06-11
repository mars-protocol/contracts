use cosmwasm_std::Coin;

pub fn get_coin(denom: &str, coins: &[Coin]) -> Coin {
    coins.iter().find(|cv| cv.denom == denom).unwrap().clone()
}
