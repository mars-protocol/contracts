use cosmwasm_std::{Addr, Coin, Deps, Storage, Uint128};

use rover::error::{ContractError, ContractResult};

use crate::state::{ALLOWED_COINS, COIN_BALANCES, RED_BANK, TOTAL_DEBT_SHARES};

pub fn assert_coin_is_whitelisted(storage: &mut dyn Storage, denom: &str) -> ContractResult<()> {
    let is_whitelisted = ALLOWED_COINS.has(storage, denom);
    if !is_whitelisted {
        return Err(ContractError::NotWhitelisted(denom.to_string()));
    }
    Ok(())
}

pub fn assert_coins_are_whitelisted(
    storage: &mut dyn Storage,
    denoms: Vec<&str>,
) -> ContractResult<()> {
    denoms
        .iter()
        .try_for_each(|denom| assert_coin_is_whitelisted(storage, denom))
}

pub fn increment_coin_balance(
    storage: &mut dyn Storage,
    account_id: &str,
    coin: &Coin,
) -> ContractResult<Uint128> {
    COIN_BALANCES.update(storage, (account_id, &coin.denom), |value_opt| {
        value_opt
            .unwrap_or_else(Uint128::zero)
            .checked_add(coin.amount)
            .map_err(ContractError::Overflow)
    })
}

pub fn decrement_coin_balance(
    storage: &mut dyn Storage,
    account_id: &str,
    coin: &Coin,
) -> ContractResult<Uint128> {
    let path = COIN_BALANCES.key((account_id, &coin.denom));
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

pub fn debt_shares_to_amount(
    deps: Deps,
    rover_addr: &Addr,
    denom: &str,
    shares: Uint128,
) -> ContractResult<Coin> {
    // total shares of debt issued for denom
    let total_debt_shares = TOTAL_DEBT_SHARES
        .load(deps.storage, denom)
        .unwrap_or(Uint128::zero());

    // total rover debt amount in Redbank for asset
    let red_bank = RED_BANK.load(deps.storage)?;
    let total_debt_amount = red_bank.query_debt(&deps.querier, rover_addr, denom)?;

    // amount of debt for token's position
    // NOTE: Given the nature of integers, the debt is rounded down. This means that the
    //       remaining share owners will take a small hit of the remainder.
    let amount = total_debt_amount.checked_multiply_ratio(shares, total_debt_shares)?;

    Ok(Coin {
        denom: denom.to_string(),
        amount,
    })
}
