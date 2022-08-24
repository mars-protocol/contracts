use std::cmp::min;

use cosmwasm_std::{Coin, DepsMut, Env, Response};

use rover::error::{ContractError, ContractResult};

use crate::state::{DEBT_SHARES, RED_BANK, TOTAL_DEBT_SHARES};
use crate::utils::{assert_coin_is_whitelisted, decrement_coin_balance};

pub fn repay(deps: DepsMut, env: Env, token_id: &str, coin: Coin) -> ContractResult<Response> {
    if coin.amount.is_zero() {
        return Err(ContractError::NoAmount);
    }

    assert_coin_is_whitelisted(deps.storage, &coin)?;

    let red_bank = RED_BANK.load(deps.storage)?;
    let total_debt_amount =
        red_bank.query_debt(&deps.querier, &env.contract.address, &coin.denom)?;

    // Calculate how many shares user is attempting to pay back
    let total_debt_shares = TOTAL_DEBT_SHARES.load(deps.storage, &coin.denom)?;
    let debt_shares_to_decrement =
        total_debt_shares.checked_multiply_ratio(coin.amount, total_debt_amount)?;

    // Payback amount should not exceed token's current debt
    let current_debt = DEBT_SHARES
        .load(deps.storage, (token_id, &coin.denom))
        .map_err(|_| ContractError::NoDebt)?;
    let shares_to_repay = min(current_debt, debt_shares_to_decrement);
    let amount_to_repay = if current_debt > debt_shares_to_decrement {
        coin.amount
    } else {
        total_debt_amount.checked_multiply_ratio(current_debt, total_debt_shares)?
    };

    // Decrement token's debt position
    if shares_to_repay >= current_debt {
        DEBT_SHARES.remove(deps.storage, (token_id, &coin.denom));
    } else {
        DEBT_SHARES.save(
            deps.storage,
            (token_id, &coin.denom),
            &current_debt.checked_sub(shares_to_repay)?,
        )?;
    }

    // Decrement total debt shares for coin
    TOTAL_DEBT_SHARES.save(
        deps.storage,
        &coin.denom,
        &total_debt_shares.checked_sub(shares_to_repay)?,
    )?;

    decrement_coin_balance(
        deps.storage,
        token_id,
        &Coin {
            denom: coin.denom.clone(),
            amount: amount_to_repay,
        },
    )?;

    let red_bank_repay_msg = red_bank.repay_msg(&Coin {
        denom: coin.denom,
        amount: amount_to_repay,
    })?;

    Ok(Response::new()
        .add_message(red_bank_repay_msg)
        .add_attribute("action", "rover/credit_manager/repay")
        .add_attribute("debt_shares_repaid", shares_to_repay)
        .add_attribute("coins_repaid", amount_to_repay))
}
