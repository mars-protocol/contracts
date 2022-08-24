use crate::state::{ALLOWED_COINS, COIN_BALANCES};
use cosmwasm_std::{Coin, Storage, Uint128};
use rover::error::{ContractError, ContractResult};

pub fn assert_coin_is_whitelisted(storage: &mut dyn Storage, coin: &Coin) -> ContractResult<()> {
    let is_whitelisted = ALLOWED_COINS.has(storage, &coin.denom);
    if !is_whitelisted {
        return Err(ContractError::NotWhitelisted(coin.denom.clone()));
    }
    Ok(())
}

pub fn increment_coin_balance(
    storage: &mut dyn Storage,
    token_id: &str,
    coin: &Coin,
) -> ContractResult<Uint128> {
    COIN_BALANCES.update(storage, (token_id, &coin.denom), |value_opt| {
        value_opt
            .unwrap_or_else(Uint128::zero)
            .checked_add(coin.amount)
            .map_err(ContractError::Overflow)
    })
}

pub fn decrement_coin_balance(
    storage: &mut dyn Storage,
    token_id: &str,
    coin: &Coin,
) -> ContractResult<Uint128> {
    let path = COIN_BALANCES.key((token_id, &coin.denom));
    let value_opt = path.may_load(storage)?;
    let new_value = value_opt
        .unwrap_or_else(Uint128::zero)
        .checked_sub(coin.amount)?;
    if new_value.is_zero() {
        path.remove(storage);
    } else {
        path.save(storage, &new_value)?;
    }
    Ok(new_value)
}
