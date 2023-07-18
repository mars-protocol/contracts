use cosmwasm_std::{Coin, Deps, DepsMut, Response, Uint128};
use mars_rover::{
    error::{ContractError, ContractResult},
    msg::execute::ActionCoin,
};

use crate::{
    state::{COIN_BALANCES, RED_BANK},
    utils::{assert_coin_is_whitelisted, decrement_coin_balance},
};

pub fn lend(mut deps: DepsMut, account_id: &str, coin: &ActionCoin) -> ContractResult<Response> {
    assert_coin_is_whitelisted(&mut deps, &coin.denom)?;

    let amount_to_lend = Coin {
        denom: coin.denom.to_string(),
        amount: get_lend_amount(deps.as_ref(), account_id, coin)?,
    };

    decrement_coin_balance(deps.storage, account_id, &amount_to_lend)?;

    let red_bank_lend_msg = RED_BANK.load(deps.storage)?.lend_msg(&amount_to_lend, account_id)?;

    Ok(Response::new()
        .add_message(red_bank_lend_msg)
        .add_attribute("action", "lend")
        .add_attribute("account_id", account_id)
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
