use cosmwasm_std::{Coin, DepsMut, Response};

use crate::{
    error::{ContractError, ContractResult},
    state::RED_BANK,
    utils::{assert_coin_is_whitelisted, increment_coin_balance},
};

pub fn borrow(mut deps: DepsMut, account_id: &str, coin: Coin) -> ContractResult<Response> {
    if coin.amount.is_zero() {
        return Err(ContractError::NoAmount);
    }

    assert_coin_is_whitelisted(&mut deps, &coin.denom)?;

    let red_bank = RED_BANK.load(deps.storage)?;

    increment_coin_balance(deps.storage, account_id, &coin)?;

    Ok(Response::new()
        .add_message(red_bank.borrow_msg(&coin, account_id)?)
        .add_attribute("action", "borrow")
        .add_attribute("account_id", account_id)
        .add_attribute("coin_borrowed", coin.to_string()))
}
