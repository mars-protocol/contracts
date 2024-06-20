use cosmwasm_std::{Coin, DepsMut, Env, Response};

use crate::{
    error::{ContractError::NoAstroLp, ContractResult},
    liquidate::calculate_liquidation,
    liquidate_deposit::repay_debt,
    state::{INCENTIVES, REWARDS_COLLECTOR},
    utils::increment_coin_balance,
};

pub fn liquidate_astro_lp(
    deps: DepsMut,
    env: Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt_coin: Coin,
    request_coin_denom: &str,
) -> ContractResult<Response> {
    let incentives = INCENTIVES.load(deps.storage)?;

    // Check how much LP coins is available for withdraw (can be withdrawn from Astro)
    let lp_position = incentives.query_staked_astro_lp_position(
        &deps.querier,
        liquidatee_account_id,
        request_coin_denom,
    )?;
    let total_lp_amount = lp_position.lp_coin.amount;

    if total_lp_amount.is_zero() {
        return Err(NoAstroLp);
    }

    let (debt, liquidator_request, liquidatee_request) = calculate_liquidation(
        &deps,
        liquidatee_account_id,
        &debt_coin,
        request_coin_denom,
        total_lp_amount,
    )?;

    // Rewards are not accounted for in the liquidation calculation (health computer includes
    // only staked astro lps in HF calculation).
    // Rewards could increase the HF (they increase deposit balance - collateral), but the impact
    // is minimal and additional complexity is not worth it.
    // We only update liquidatee's balance with rewards.
    for reward in lp_position.rewards.iter() {
        increment_coin_balance(deps.storage, liquidatee_account_id, reward)?;
    }

    // Liquidator pays down debt on behalf of liquidatee
    let repay_msg =
        repay_debt(deps.storage, &env, liquidator_account_id, liquidatee_account_id, &debt)?;

    // Liquidatee's LP coin withdrawn from Astro
    let withdraw_from_liquidatee_msg =
        incentives.unstake_astro_lp_msg(liquidatee_account_id, &liquidatee_request)?;

    // Liquidator gets portion of withdrawn LP coin
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
        .add_message(withdraw_from_liquidatee_msg)
        .add_attribute("action", "liquidate_astro_lp")
        .add_attribute("account_id", liquidator_account_id)
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("coin_debt_repaid", debt.to_string())
        .add_attribute("coin_liquidated", liquidatee_request.to_string())
        .add_attribute("protocol_fee_coin", protocol_fee_coin.to_string()))
}
