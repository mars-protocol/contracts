use std::cmp::min;

use cosmwasm_std::{Coin, Deps, DepsMut, Env, Response, Uint128};
use mars_rover::{
    error::{ContractError, ContractResult},
    msg::execute::ActionCoin,
};

use crate::{
    state::{LENT_SHARES, RED_BANK, TOTAL_LENT_SHARES},
    utils::{increment_coin_balance, lent_shares_to_amount},
};

pub fn reclaim(
    deps: DepsMut,
    env: Env,
    account_id: &str,
    coin: &ActionCoin,
) -> ContractResult<Response> {
    let (lent_amount, lent_shares) =
        current_lent_amount_for_denom(deps.as_ref(), &env, account_id, &coin.denom)?;
    let amount_to_reclaim = min(lent_amount, coin.amount.value().unwrap_or(Uint128::MAX));
    let shares_to_reclaim = lent_amount_to_shares(
        deps.as_ref(),
        &env,
        &Coin {
            denom: coin.denom.to_string(),
            amount: amount_to_reclaim,
        },
    )?;

    // Decrement token's lent position
    if amount_to_reclaim == lent_amount {
        LENT_SHARES.remove(deps.storage, (account_id, &coin.denom));
    } else {
        LENT_SHARES.save(
            deps.storage,
            (account_id, &coin.denom),
            &lent_shares.checked_sub(shares_to_reclaim)?,
        )?;
    }

    // Decrement total lent shares for coin
    let total_lent_shares = TOTAL_LENT_SHARES.load(deps.storage, &coin.denom)?;
    TOTAL_LENT_SHARES.save(
        deps.storage,
        &coin.denom,
        &total_lent_shares.checked_sub(shares_to_reclaim)?,
    )?;

    increment_coin_balance(
        deps.storage,
        account_id,
        &Coin {
            denom: coin.denom.to_string(),
            amount: amount_to_reclaim,
        },
    )?;

    let red_bank = RED_BANK.load(deps.storage)?;
    let red_bank_reclaim_msg = red_bank.reclaim_msg(&Coin {
        denom: coin.denom.to_string(),
        amount: amount_to_reclaim,
    })?;

    Ok(Response::new()
        .add_message(red_bank_reclaim_msg)
        .add_attribute("action", "reclaim")
        .add_attribute("lent_shares_reclaimed", shares_to_reclaim)
        .add_attribute("coin_reclaimed", format!("{}{}", amount_to_reclaim, &coin.denom)))
}

pub fn lent_amount_to_shares(deps: Deps, env: &Env, coin: &Coin) -> ContractResult<Uint128> {
    let red_bank = RED_BANK.load(deps.storage)?;
    let total_lent_shares = TOTAL_LENT_SHARES.load(deps.storage, &coin.denom)?;
    let total_lent = red_bank.query_lent(&deps.querier, &env.contract.address, &coin.denom)?;
    let shares = total_lent_shares.checked_multiply_ratio(coin.amount, total_lent)?;
    Ok(shares)
}

/// Get token's current lent amount for denom
/// Returns -> (lent amount, lent shares)
pub fn current_lent_amount_for_denom(
    deps: Deps,
    env: &Env,
    account_id: &str,
    denom: &str,
) -> ContractResult<(Uint128, Uint128)> {
    let lent_shares =
        LENT_SHARES.load(deps.storage, (account_id, denom)).map_err(|_| ContractError::NoneLent)?;
    let coin = lent_shares_to_amount(deps, &env.contract.address, denom, lent_shares)?;
    Ok((coin.amount, lent_shares))
}
