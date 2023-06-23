use cosmwasm_std::{Coin, DepsMut, Env, Response};
use mars_rover::error::ContractResult;

use crate::{
    liquidate::calculate_liquidation,
    liquidate_deposit::repay_debt,
    reclaim::{current_lent_amount_for_denom, lent_amount_to_shares},
    state::REWARDS_COLLECTOR,
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

    let (debt, liquidator_request, liquidatee_request) = calculate_liquidation(
        &deps,
        &env,
        liquidatee_account_id,
        &debt_coin,
        request_coin_denom,
        total_lent_amount,
    )?;

    let repay_msg =
        repay_debt(deps.storage, &env, liquidator_account_id, liquidatee_account_id, &debt)?;

    let shares_from_liquidatee = lent_amount_to_shares(
        deps.as_ref(),
        &env,
        &Coin {
            denom: request_coin_denom.to_string(),
            amount: liquidatee_request.amount,
        },
    )?;
    let shares_to_liquidator = lent_amount_to_shares(
        deps.as_ref(),
        &env,
        &Coin {
            denom: request_coin_denom.to_string(),
            amount: liquidator_request.amount,
        },
    )?;

    decrement_lent_shares(
        deps.storage,
        liquidatee_account_id,
        request_coin_denom,
        shares_from_liquidatee,
    )?;
    increment_lent_shares(
        deps.storage,
        liquidator_account_id,
        request_coin_denom,
        shares_to_liquidator,
    )?;

    // Transfer protocol fee to rewards-collector account
    let rewards_collector_account = REWARDS_COLLECTOR.load(deps.storage)?.account_id;
    let protocol_fee_shares = shares_from_liquidatee.checked_sub(shares_to_liquidator)?;
    increment_lent_shares(
        deps.storage,
        &rewards_collector_account,
        request_coin_denom,
        protocol_fee_shares,
    )?;

    Ok(Response::new()
        .add_message(repay_msg)
        .add_attribute("action", "liquidate_lend")
        .add_attribute("account_id", liquidator_account_id)
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("coin_debt_repaid", debt.to_string())
        .add_attribute("coin_liquidated", liquidatee_request.to_string())
        .add_attribute(
            "protocol_fee_coin",
            Coin::new(protocol_fee_shares.u128(), request_coin_denom).to_string(),
        ))
}
