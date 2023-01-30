use cosmwasm_std::{Coin, DepsMut, Env, Response, Uint128};
use mars_rover::error::{ContractError, ContractResult};

use crate::{
    state::{LENT_SHARES, RED_BANK, TOTAL_LENT_SHARES},
    utils::{assert_coin_is_whitelisted, decrement_coin_balance},
};

pub static DEFAULT_LENT_SHARES_PER_COIN: Uint128 = Uint128::new(1_000_000);

pub fn lend(deps: DepsMut, env: Env, account_id: &str, coin: Coin) -> ContractResult<Response> {
    if coin.amount.is_zero() {
        return Err(ContractError::NoAmount);
    }

    assert_coin_is_whitelisted(deps.storage, &coin.denom)?;

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

    decrement_coin_balance(deps.storage, account_id, &coin)?;

    Ok(Response::new()
        .add_message(red_bank.lend_msg(&coin)?)
        .add_attribute("action", "lend")
        .add_attribute("account_id", account_id)
        .add_attribute("lent_shares_added", lent_shares_to_add)
        .add_attribute("coins_lent", coin.amount))
}
