use cosmwasm_std::{Coin, Deps, DepsMut, Env, Response, Uint128};
use mars_rover::{
    error::{ContractError, ContractResult},
    msg::execute::ActionCoin,
};

use crate::{
    state::{COIN_BALANCES, LENT_SHARES, RED_BANK, TOTAL_LENT_SHARES},
    utils::{assert_coin_is_whitelisted, decrement_coin_balance},
};

pub static DEFAULT_LENT_SHARES_PER_COIN: Uint128 = Uint128::new(1_000_000);

pub fn lend(
    mut deps: DepsMut,
    env: Env,
    account_id: &str,
    coin: &ActionCoin,
) -> ContractResult<Response> {
    assert_coin_is_whitelisted(&mut deps, &coin.denom)?;

    let amount_to_lend = Coin {
        denom: coin.denom.to_string(),
        amount: get_lend_amount(deps.as_ref(), account_id, coin)?,
    };

    // Total Credit Manager has lent to Red Bank for denom
    let red_bank = RED_BANK.load(deps.storage)?;
    let total_lent =
        red_bank.query_lent(&deps.querier, &env.contract.address, &amount_to_lend.denom)?;

    let lent_shares_to_add = if total_lent.is_zero() {
        amount_to_lend.amount.checked_mul(DEFAULT_LENT_SHARES_PER_COIN)?
    } else {
        TOTAL_LENT_SHARES
            .load(deps.storage, &amount_to_lend.denom)?
            .checked_multiply_ratio(amount_to_lend.amount, total_lent)?
    };

    let add_shares = |shares: Option<Uint128>| -> ContractResult<Uint128> {
        Ok(shares.unwrap_or_else(Uint128::zero).checked_add(lent_shares_to_add)?)
    };

    TOTAL_LENT_SHARES.update(deps.storage, &amount_to_lend.denom, add_shares)?;
    LENT_SHARES.update(deps.storage, (account_id, &amount_to_lend.denom), add_shares)?;

    decrement_coin_balance(deps.storage, account_id, &amount_to_lend)?;

    let red_bank_lend_msg = red_bank.lend_msg(&amount_to_lend)?;

    Ok(Response::new()
        .add_message(red_bank_lend_msg)
        .add_attribute("action", "lend")
        .add_attribute("account_id", account_id)
        .add_attribute("lent_shares_added", lent_shares_to_add)
        .add_attribute("coin_lent", &amount_to_lend.denom))
}

/// Queries balance to ensure passing EXACT is not too high.
/// Also asserts the amount is greater than zero.
fn get_lend_amount(deps: Deps, account_id: &str, coin: &ActionCoin) -> ContractResult<Uint128> {
    let amount_to_lend = if let Some(amount) = coin.amount.value() {
        amount
    } else {
        COIN_BALANCES.may_load(deps.storage, (account_id, &coin.denom))?.unwrap_or(Uint128::zero())
    };

    if amount_to_lend.is_zero() {
        Err(ContractError::NoAmount)
    } else {
        Ok(amount_to_lend)
    }
}
