use cosmwasm_std::{Coin, DepsMut, Env, Response};
use mars_rover::error::{ContractError::NoneLent, ContractResult};

use crate::{
    liquidate::calculate_liquidation,
    liquidate_deposit::repay_debt,
    state::{RED_BANK, REWARDS_COLLECTOR},
    utils::increment_coin_balance,
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
    let total_lent_amount = RED_BANK.load(deps.storage)?.query_lent(
        &deps.querier,
        liquidatee_account_id,
        request_coin_denom,
    )?;

    if total_lent_amount.is_zero() {
        return Err(NoneLent);
    }

    let (debt, liquidator_request, liquidatee_request) = calculate_liquidation(
        &deps,
        liquidatee_account_id,
        &debt_coin,
        request_coin_denom,
        total_lent_amount,
    )?;

    // Liquidator pays down debt on behalf of liquidatee
    let repay_msg =
        repay_debt(deps.storage, &env, liquidator_account_id, liquidatee_account_id, &debt)?;

    // Liquidatee's lent coin reclaimed from Red Bank
    let red_bank = RED_BANK.load(deps.storage)?;
    let reclaim_from_liquidatee_msg =
        red_bank.reclaim_msg(&liquidatee_request, liquidatee_account_id)?;

    // Liquidator gets portion of reclaimed lent coin
    increment_coin_balance(deps.storage, liquidator_account_id, &liquidator_request)?;

    // Transfer protocol fee to rewards-collector account
    let rewards_collector_account = REWARDS_COLLECTOR.load(deps.storage)?.account_id;
    let protocol_fee_coin = Coin {
        denom: request_coin_denom.to_string(),
        amount: liquidatee_request.amount.checked_sub(liquidator_request.amount)?,
    };
    increment_coin_balance(deps.storage, &rewards_collector_account, &protocol_fee_coin)?;

    Ok(Response::new()
        .add_message(repay_msg)
        .add_message(reclaim_from_liquidatee_msg)
        .add_attribute("action", "liquidate_lend")
        .add_attribute("account_id", liquidator_account_id)
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("coin_debt_repaid", debt.to_string())
        .add_attribute("coin_liquidated", liquidatee_request.to_string())
        .add_attribute("protocol_fee_coin", protocol_fee_coin.to_string()))
}
