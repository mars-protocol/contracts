use cosmwasm_std::{Coin, Uint128};

use crate::error::ContractError;

// native coins
pub fn get_denom_amount_from_coins(coins: &[Coin], denom: &str) -> Result<Uint128, ContractError> {
    if coins.len() == 1 && coins[0].denom == denom {
        Ok(coins[0].amount)
    } else {
        Err(ContractError::InvalidCoinsSent {
            denom: denom.to_string(),
        })
    }
}
