use cosmwasm_std::{Coin, DepsMut, Env, Response};
use mars_rover::error::ContractResult;

use crate::{
    liquidate_deposit::{calculate_liquidation, repay_debt},
    reclaim::{current_lent_amount_for_denom, lent_amount_to_shares},
    utils::{decrement_lent_shares, increment_lent_shares},
};

pub fn liquidate_lend(
    deps: DepsMut,
    env: Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt_coin: Coin,
    request_coin_denom: &str,
) -> ContractResult<Response> {
    // Check how much lent coin is available for reclaim (can be withdrawn from Red Bank)
    let (total_lent_amount, _) = current_lent_amount_for_denom(
        deps.as_ref(),
        &env,
        liquidatee_account_id,
        request_coin_denom,
    )?;

    let (debt, request) = calculate_liquidation(
        &deps,
        &env,
        liquidatee_account_id,
        &debt_coin,
        request_coin_denom,
        total_lent_amount,
    )?;

    let repay_msg =
        repay_debt(deps.storage, &env, liquidator_account_id, liquidatee_account_id, &debt)?;

    let shares_to_transfer = lent_amount_to_shares(
        deps.as_ref(),
        &env,
        &Coin {
            denom: request_coin_denom.to_string(),
            amount: request.amount,
        },
    )?;

    // Transfer requested lent coin from liquidatee to liquidator
    decrement_lent_shares(
        deps.storage,
        liquidatee_account_id,
        request_coin_denom,
        shares_to_transfer,
    )?;
    increment_lent_shares(
        deps.storage,
        liquidator_account_id,
        request_coin_denom,
        shares_to_transfer,
    )?;

    Ok(Response::new()
        .add_message(repay_msg)
        .add_attribute("action", "liquidate_lend")
        .add_attribute("account_id", liquidator_account_id)
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("coin_debt_repaid", debt.to_string())
        .add_attribute("coin_liquidated", request.to_string()))
}
