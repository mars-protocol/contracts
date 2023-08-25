use std::cmp::min;

use cosmwasm_std::{Coin, DepsMut, Response, Uint128};
use mars_rover::{
    error::{ContractError::NoneLent, ContractResult},
    msg::execute::ActionCoin,
};

use crate::{state::RED_BANK, utils::increment_coin_balance};

pub fn reclaim(deps: DepsMut, account_id: &str, coin: &ActionCoin) -> ContractResult<Response> {
    let red_bank = RED_BANK.load(deps.storage)?;
    let lent_amount = red_bank.query_lent(&deps.querier, account_id, &coin.denom)?;
    let amount_to_reclaim = min(lent_amount, coin.amount.value().unwrap_or(Uint128::MAX));

    if amount_to_reclaim.is_zero() {
        return Err(NoneLent);
    }

    increment_coin_balance(
        deps.storage,
        account_id,
        &Coin {
            denom: coin.denom.to_string(),
            amount: amount_to_reclaim,
        },
    )?;

    let red_bank_reclaim_msg = red_bank.reclaim_msg(
        &Coin {
            denom: coin.denom.to_string(),
            amount: amount_to_reclaim,
        },
        account_id,
        false,
    )?;

    Ok(Response::new()
        .add_message(red_bank_reclaim_msg)
        .add_attribute("action", "reclaim")
        .add_attribute("account_id", account_id)
        .add_attribute("coin_reclaimed", format!("{}{}", amount_to_reclaim, &coin.denom)))
}
