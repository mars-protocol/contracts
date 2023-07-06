use cosmwasm_std::{Coin, DepsMut, Env, Response, Storage, Uint128};
use mars_rover::error::{ContractError, ContractResult};

use crate::{
    state::{LENT_SHARES, RED_BANK, TOTAL_LENT_SHARES},
    utils::{assert_coin_is_whitelisted, decrement_coin_balance},
};

pub static DEFAULT_LENT_SHARES_PER_COIN: Uint128 = Uint128::new(1_000_000);

pub fn lend(mut deps: DepsMut, env: Env, account_id: &str, coin: Coin) -> ContractResult<Response> {
    if coin.amount.is_zero() {
        return Err(ContractError::NoAmount);
    }

    assert_coin_is_whitelisted(&mut deps, &coin.denom)?;

    let red_bank = RED_BANK.load(deps.storage)?;
    let total_lent = red_bank.query_lent(&deps.querier, &env.contract.address, &coin.denom)?;

    let lent_shares_to_add = if total_lent.is_zero() {
        coin.amount.checked_mul(DEFAULT_LENT_SHARES_PER_COIN)?
    } else {
        TOTAL_LENT_SHARES
            .load(deps.storage, &coin.denom)?
            .checked_multiply_ratio(coin.amount, total_lent)?
    };

    let add_shares = |shares: Option<Uint128>| -> ContractResult<Uint128> {
        Ok(shares.unwrap_or_else(Uint128::zero).checked_add(lent_shares_to_add)?)
    };

    TOTAL_LENT_SHARES.update(deps.storage, &coin.denom, add_shares)?;
    LENT_SHARES.update(deps.storage, (account_id, &coin.denom), add_shares)?;

    assert_lend_amount(deps.storage, account_id, &coin, total_lent)?;
    decrement_coin_balance(deps.storage, account_id, &coin)?;

    Ok(Response::new()
        .add_message(red_bank.lend_msg(&coin)?)
        .add_attribute("action", "lend")
        .add_attribute("account_id", account_id)
        .add_attribute("lent_shares_added", lent_shares_to_add)
        .add_attribute("coin_lent", coin.to_string()))
}

/// A guard to ensure once a user makes a lend, the amount they can reclaim is >= 1.
/// Due to integer rounding, if the pool shares issued are quite large and the lend action
/// amount is low, it could round down to zero.
fn assert_lend_amount(
    storage: &dyn Storage,
    account_id: &str,
    coin_to_lend: &Coin,
    total_lent: Uint128,
) -> ContractResult<()> {
    let total_shares = TOTAL_LENT_SHARES.load(storage, &coin_to_lend.denom)?;
    let user_shares = LENT_SHARES.load(storage, (account_id, &coin_to_lend.denom))?;

    if total_lent
        .checked_add(coin_to_lend.amount)?
        .checked_mul_floor((user_shares, total_shares))?
        .is_zero()
    {
        return Err(ContractError::NoAmount);
    }
    Ok(())
}
