use cosmwasm_std::{coin, DepsMut, Response};
use mars_types::{
    credit_manager::{ActionAmount, ActionCoin},
    traits::Stringify,
};

use crate::{
    error::{ContractError, ContractResult},
    state::INCENTIVES,
    utils::increment_coin_balance,
};

pub fn unstake_lp(
    deps: DepsMut,
    account_id: &str,
    lp_coin: ActionCoin,
) -> ContractResult<Response> {
    let incentives = INCENTIVES.load(deps.storage)?;

    // Query rewards user is receiving, update balance
    let lp_position = incentives.query_staked_astro_lp_position(
        &deps.querier,
        account_id,
        lp_coin.denom.as_str(),
    )?;

    for reward in lp_position.rewards.iter() {
        increment_coin_balance(deps.storage, account_id, reward)?;
    }

    let amount_to_unstake = match lp_coin.amount {
        ActionAmount::Exact(amt) => {
            if lp_position.lp_coin.amount.lt(&amt) {
                return Err(ContractError::InsufficientFunds {
                    requested: amt,
                    available: lp_position.lp_coin.amount,
                });
            } else {
                amt
            }
        }
        ActionAmount::AccountBalance => lp_position.lp_coin.amount,
    };

    let updated_coin = coin(amount_to_unstake.u128(), lp_coin.denom.as_str());

    increment_coin_balance(deps.storage, account_id, &updated_coin)?;

    // unstake msg
    let unstake_msg = incentives.unstake_astro_lp_msg(account_id, &updated_coin)?;

    let mut res = Response::new()
        .add_message(unstake_msg)
        .add_attribute("action", "unstake_astro_lp")
        .add_attribute("account_id", account_id)
        .add_attribute("lp_unstaked", updated_coin.to_string());

    if !lp_position.rewards.is_empty() {
        res = res.add_attribute("rewards", lp_position.rewards.as_slice().to_string());
    }

    Ok(res)
}
