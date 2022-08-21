use std::collections::HashMap;

use cosmwasm_std::{Addr, Decimal, Deps, Env, StdError, StdResult, Uint128};
use mars_health::health::{Health, Position as HealthPosition};
use mars_outpost::helpers::cw20_get_balance;
use mars_outpost::oracle;
use mars_outpost::red_bank::{Debt, Position, User};

use crate::helpers::{get_bit, get_market_from_index};
use crate::interest_rates::{get_underlying_debt_amount, get_underlying_liquidity_amount};
use crate::state::{DEBTS, GLOBAL_STATE};

/// Check the Health Factor for a given user
pub fn assert_liquidatable(
    deps: &Deps,
    env: &Env,
    user: &User,
    user_addr: &Addr,
    oracle_addr: &Addr,
) -> StdResult<(bool, HashMap<String, Position>)> {
    let positions = get_user_positions_map(deps, env, user, user_addr, oracle_addr)?;
    let health = compute_position_health(&positions)?;

    Ok((health.is_liquidatable(), positions))
}

/// Check the Health Factor for a given user after a withdraw
pub fn assert_health_after_withdraw(
    deps: &Deps,
    env: &Env,
    user: &User,
    user_addr: &Addr,
    oracle_addr: &Addr,
    denom: &str,
    amount: Uint128,
) -> StdResult<bool> {
    let mut positions = get_user_positions_map(deps, env, user, user_addr, oracle_addr)?;

    // Update position to compute health factor after withdraw
    positions
        .get_mut(denom)
        .ok_or(StdError::GenericErr {
            msg: "No User Balance".to_string(),
        })?
        .collateral_amount -= amount;

    let health = compute_position_health(&positions)?;
    Ok(!health.is_liquidatable())
}

/// Check the Health Factor for a given user after a borrow
pub fn assert_health_after_borrow(
    deps: &Deps,
    env: &Env,
    user: &User,
    user_addr: &Addr,
    oracle_addr: &Addr,
    denom: &str,
    amount: Uint128,
) -> StdResult<bool> {
    let mut positions = get_user_positions_map(deps, env, user, user_addr, oracle_addr)?;

    // Update position to compute health factor after borrow
    positions
        .entry(denom.to_string())
        .or_insert(Position {
            denom: denom.to_string(),
            debt_amount: Uint128::zero(),
            asset_price: oracle::helpers::query_price(&deps.querier, oracle_addr, denom)?,
            ..Default::default()
        })
        .debt_amount += amount;

    let health = compute_position_health(&positions)?;
    Ok(!health.is_above_max_ltv())
}

/// Assert Health of a given User Position
pub fn compute_position_health(positions: &HashMap<String, Position>) -> StdResult<Health> {
    let positions = positions
        .values()
        .map(|p| {
            let debt_amount = if p.uncollateralized_debt {
                Decimal::zero()
            } else {
                Decimal::from_ratio(p.debt_amount, 1u128)
            };

            HealthPosition {
                denom: p.denom.clone(),
                collateral_amount: Decimal::from_ratio(p.collateral_amount, 1u128),
                debt_amount,
                price: p.asset_price,
                max_ltv: p.max_ltv,
                liquidation_threshold: p.liquidation_threshold,
            }
        })
        .collect::<Vec<_>>();

    Health::compute_health(&positions)
}

/// Goes through assets user has a position in and returns a vec containing the scaled debt
/// (denominated in the asset), a result from a specified computation for the current collateral
/// (denominated in asset) and some metadata to be used by the caller.
pub fn get_user_positions(
    deps: &Deps,
    env: &Env,
    user: &User,
    user_addr: &Addr,
    oracle_addr: &Addr,
) -> StdResult<Vec<Position>> {
    let mut ret: Vec<Position> = vec![];
    let global_state = GLOBAL_STATE.load(deps.storage)?;

    for i in 0_u32..global_state.market_count {
        let user_is_using_as_collateral = get_bit(user.collateral_assets, i)?;
        let user_is_borrowing = get_bit(user.borrowed_assets, i)?;
        if !(user_is_using_as_collateral || user_is_borrowing) {
            continue;
        }

        let (denom, market) = get_market_from_index(deps, i)?;

        let (collateral_amount, max_ltv, liquidation_threshold) = if user_is_using_as_collateral {
            // query asset balance (ma_token contract gives back a scaled value)
            let asset_balance_scaled = cw20_get_balance(
                &deps.querier,
                market.ma_token_address.clone(),
                user_addr.clone(),
            )?;

            let collateral_amount = get_underlying_liquidity_amount(
                asset_balance_scaled,
                &market,
                env.block.time.seconds(),
            )?;

            (collateral_amount, market.max_loan_to_value, market.liquidation_threshold)
        } else {
            (Uint128::zero(), Decimal::zero(), Decimal::zero())
        };

        let (debt_amount, uncollateralized_debt) = if user_is_borrowing {
            // query debt
            let user_debt: Debt = DEBTS.load(deps.storage, (&denom, user_addr))?;

            let debt_amount = get_underlying_debt_amount(
                user_debt.amount_scaled,
                &market,
                env.block.time.seconds(),
            )?;

            (debt_amount, user_debt.uncollateralized)
        } else {
            (Uint128::zero(), false)
        };

        let asset_price = oracle::helpers::query_price(&deps.querier, oracle_addr, &denom)?;

        let user_asset_position = Position {
            denom,
            collateral_amount,
            debt_amount,
            uncollateralized_debt,
            max_ltv,
            liquidation_threshold,
            asset_price,
        };
        ret.push(user_asset_position);
    }

    Ok(ret)
}

pub fn get_user_positions_map(
    deps: &Deps,
    env: &Env,
    user: &User,
    user_addr: &Addr,
    oracle_addr: &Addr,
) -> StdResult<HashMap<String, Position>> {
    Ok(get_user_positions(deps, env, user, user_addr, oracle_addr)?
        .into_iter()
        .map(|p| (p.denom.clone(), p))
        .collect())
}
