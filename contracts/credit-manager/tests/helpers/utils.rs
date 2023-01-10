use cosmwasm_std::Coin;
use mars_rover::msg::query::DebtAmount;

pub fn get_coin(denom: &str, coins: &[Coin]) -> Coin {
    coins.iter().find(|cv| cv.denom == denom).unwrap().clone()
}

pub fn get_debt(denom: &str, coins: &[DebtAmount]) -> DebtAmount {
    coins.iter().find(|coin| coin.denom.as_str() == denom).unwrap().clone()
}
