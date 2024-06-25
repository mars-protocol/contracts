use cosmwasm_std::{coin, DepsMut, Response};
use mars_types::{
    credit_manager::{ActionAmount, ActionCoin},
    traits::Stringify,
};

use crate::{
    error::{ContractError, ContractResult},
    state::{COIN_BALANCES, INCENTIVES},
    utils::{decrement_coin_balance, increment_coin_balance},
};

pub fn stake_lp(deps: DepsMut, account_id: &str, lp_coin: ActionCoin) -> ContractResult<Response> {
    let incentives = INCENTIVES.load(deps.storage)?;

    // Query rewards user is receiving to update their balances
    let rewards = incentives.query_staked_astro_lp_rewards(
        &deps.querier,
        account_id,
        lp_coin.denom.as_str(),
    )?;

    let coin_balance =
        COIN_BALANCES.may_load(deps.storage, (account_id, &lp_coin.denom))?.unwrap_or_default();
    let new_amount = match lp_coin.amount {
        ActionAmount::Exact(amt) => amt,
        ActionAmount::AccountBalance => coin_balance,
    };

    if new_amount.is_zero() {
        return Err(ContractError::NoAmount);
    };

    let updated_coin = coin(new_amount.u128(), lp_coin.denom.as_str());

    decrement_coin_balance(deps.storage, account_id, &updated_coin)?;

    for reward in rewards.iter() {
        increment_coin_balance(deps.storage, account_id, reward)?;
    }

    // stake msg
    let stake_msg = incentives.stake_astro_lp_msg(account_id, updated_coin)?;
    let mut res = Response::new()
        .add_message(stake_msg)
        .add_attribute("action", "stake_astro_lp")
        .add_attribute("account_id", account_id);

    if !rewards.is_empty() {
        res = res.add_attribute("rewards", rewards.as_slice().to_string());
    }

    Ok(res)
}
