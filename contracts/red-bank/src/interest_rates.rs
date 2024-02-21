use std::str;

use cosmwasm_std::{Addr, Decimal, Env, Event, Response, Storage, Uint128};
use mars_interest_rate::{
    calculate_applied_linear_interest_rate, compute_scaled_amount, compute_underlying_amount,
    get_underlying_debt_amount, get_underlying_liquidity_amount, ScalingOperation,
};
use mars_types::red_bank::Market;

use crate::{error::ContractError, user::User};

/// Calculates accumulated interest for the time between last time market index was updated
/// and current block.
/// Applies desired side effects:
/// 1. Updates market borrow and liquidity indices.
/// 2. If there are any protocol rewards, builds a mint to the rewards collector and adds it
///    to the returned response
/// NOTE: it does not save the market to store
/// WARNING: For a given block, this function should be called before updating interest rates
/// as it would apply the new interest rates instead of the ones that were valid during
/// the period between indexes_last_updated and current_block
pub fn apply_accumulated_interests(
    store: &mut dyn Storage,
    env: &Env,
    market: &mut Market,
    rewards_collector_addr: &Addr,
    incentives_addr: &Addr,
    mut response: Response,
) -> Result<Response, ContractError> {
    let current_timestamp = env.block.time.seconds();
    let previous_borrow_index = market.borrow_index;

    // Update market indices
    if market.indexes_last_updated < current_timestamp {
        let time_elapsed = current_timestamp - market.indexes_last_updated;

        if !market.borrow_rate.is_zero() {
            market.borrow_index = calculate_applied_linear_interest_rate(
                market.borrow_index,
                market.borrow_rate,
                time_elapsed,
            )?;
        }
        if !market.liquidity_rate.is_zero() {
            market.liquidity_index = calculate_applied_linear_interest_rate(
                market.liquidity_index,
                market.liquidity_rate,
                time_elapsed,
            )?;
        }
        market.indexes_last_updated = current_timestamp;
    }

    // Compute accrued protocol rewards
    let previous_debt_total = compute_underlying_amount(
        market.debt_total_scaled,
        previous_borrow_index,
        ScalingOperation::Ceil,
    )?;
    let new_debt_total = compute_underlying_amount(
        market.debt_total_scaled,
        market.borrow_index,
        ScalingOperation::Ceil,
    )?;

    let borrow_interest_accrued = if new_debt_total > previous_debt_total {
        // debt stays constant between the application of the interest rate
        // so the difference between debt at the start and the end is the
        // total borrow interest accrued
        new_debt_total - previous_debt_total
    } else {
        Uint128::zero()
    };

    let accrued_protocol_rewards = borrow_interest_accrued * market.reserve_factor;

    if !accrued_protocol_rewards.is_zero() {
        let reward_amount_scaled = compute_scaled_amount(
            accrued_protocol_rewards,
            market.liquidity_index,
            ScalingOperation::Truncate,
        )?;
        response = User(rewards_collector_addr).increase_collateral(
            store,
            market,
            reward_amount_scaled,
            incentives_addr,
            response,
            None,
        )?;
        market.increase_collateral(reward_amount_scaled)?;
    }

    Ok(response)
}

/// Update interest rates for current liquidity and debt levels
/// Note it does not save the market to the store (that is left to the caller)
/// Returns response with appended interest rates updated event
/// NOTE: For a given block, interest rates should not be updated before updating indexes first
/// as it should result in wrong indexes
pub fn update_interest_rates(
    env: &Env,
    market: &mut Market,
    response: Response,
) -> Result<Response, ContractError> {
    let current_timestamp = env.block.time.seconds();

    let total_collateral =
        get_underlying_liquidity_amount(market.collateral_total_scaled, market, current_timestamp)?;
    let total_debt =
        get_underlying_debt_amount(market.debt_total_scaled, market, current_timestamp)?;

    let mut current_utilization_rate = if !total_collateral.is_zero() {
        Decimal::from_ratio(total_debt, total_collateral)
    } else {
        Decimal::zero()
    };

    // Limit utilization_rate to 100%.
    // With the current code it should hopefully never happen that it gets calculated to more than 100%,
    // but better be safe than sorry.
    current_utilization_rate = current_utilization_rate.min(Decimal::one());

    market.update_interest_rates(current_utilization_rate)?;

    Ok(response.add_event(build_interests_updated_event(&market.denom, market)))
}

pub fn build_interests_updated_event(denom: &str, market: &Market) -> Event {
    Event::new("interests_updated")
        .add_attribute("denom", denom)
        .add_attribute("borrow_index", market.borrow_index.to_string())
        .add_attribute("liquidity_index", market.liquidity_index.to_string())
        .add_attribute("borrow_rate", market.borrow_rate.to_string())
        .add_attribute("liquidity_rate", market.liquidity_rate.to_string())
}
