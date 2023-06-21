#[cfg(feature = "backtraces")]
use std::backtrace::Backtrace;
use std::collections::{HashMap, HashSet};

use cosmwasm_std::{Addr, Deps, Env, Order, StdError, StdResult, Uint128};
use mars_health::health::{Health, Position as HealthPosition};
use mars_red_bank_types::{oracle, red_bank::Position};

use crate::{
    error::ContractError,
    helpers::query_asset_params,
    interest_rates::{get_underlying_debt_amount, get_underlying_liquidity_amount},
    state::{COLLATERALS, DEBTS, MARKETS},
};

/// Check the Health Factor for a given user
pub fn assert_liquidatable(
    deps: &Deps,
    env: &Env,
    user_addr: &Addr,
    oracle_addr: &Addr,
    params_addr: &Addr,
) -> Result<(bool, HashMap<String, Position>), ContractError> {
    let positions = get_user_positions_map(deps, env, user_addr, oracle_addr, params_addr)?;
    let health = compute_position_health(&positions)?;

    Ok((health.is_liquidatable(), positions))
}

/// Check the Health Factor for a given user after a withdraw
pub fn assert_below_liq_threshold_after_withdraw(
    deps: &Deps,
    env: &Env,
    user_addr: &Addr,
    oracle_addr: &Addr,
    params_addr: &Addr,
    denom: &str,
    withdraw_amount: Uint128,
) -> Result<bool, ContractError> {
    let mut positions = get_user_positions_map(deps, env, user_addr, oracle_addr, params_addr)?;

    // Update position to compute health factor after withdraw
    match positions.get_mut(denom) {
        Some(p) => {
            p.collateral_amount = p.collateral_amount.checked_sub(withdraw_amount)?;
        }
        None => {
            return Err(StdError::GenericErr {
                msg: "No User Balance".to_string(),
                #[cfg(feature = "backtraces")]
                backtrace: Backtrace::capture(),
            }
            .into())
        }
    }

    let health = compute_position_health(&positions)?;
    Ok(!health.is_liquidatable())
}

/// Check the Health Factor for a given user after a borrow
pub fn assert_below_max_ltv_after_borrow(
    deps: &Deps,
    env: &Env,
    user_addr: &Addr,
    oracle_addr: &Addr,
    params_addr: &Addr,
    denom: &str,
    borrow_amount: Uint128,
) -> Result<bool, ContractError> {
    let mut positions = get_user_positions_map(deps, env, user_addr, oracle_addr, params_addr)?;

    // Update position to compute health factor after borrow
    positions
        .entry(denom.to_string())
        .or_insert(Position {
            denom: denom.to_string(),
            debt_amount: Uint128::zero(),
            asset_price: oracle::helpers::query_price(&deps.querier, oracle_addr, denom)?,
            ..Default::default()
        })
        .debt_amount += borrow_amount;

    let health = compute_position_health(&positions)?;
    Ok(!health.is_above_max_ltv())
}

/// Compute Health of a given User Position
pub fn compute_position_health(
    positions: &HashMap<String, Position>,
) -> Result<Health, ContractError> {
    let positions = positions
        .values()
        .map(|p| {
            // if it is an "uncollateralized" debt, then it won't count towards their health factor
            let debt_amount = if p.uncollateralized_debt {
                Uint128::zero()
            } else {
                p.debt_amount
            };

            HealthPosition {
                denom: p.denom.clone(),
                collateral_amount: p.collateral_amount,
                debt_amount,
                price: p.asset_price,
                max_ltv: p.max_ltv,
                liquidation_threshold: p.liquidation_threshold,
            }
        })
        .collect::<Vec<_>>();

    Health::compute_health(&positions).map_err(Into::into)
}

/// Goes through assets user has a position in and returns a HashMap mapping the asset denoms to the
/// scaled amounts, and some metadata to be used by the caller.
pub fn get_user_positions_map(
    deps: &Deps,
    env: &Env,
    user_addr: &Addr,
    oracle_addr: &Addr,
    params_addr: &Addr,
) -> StdResult<HashMap<String, Position>> {
    let block_time = env.block.time.seconds();

    // Find all denoms that the user has a collateral or debt position in
    let collateral_denoms = COLLATERALS
        .prefix(user_addr)
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    let debt_denoms = DEBTS
        .prefix(user_addr)
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    // Collect the denoms into a hashset so that there are no dups
    let mut denoms = HashSet::new();
    denoms.extend(collateral_denoms);
    denoms.extend(debt_denoms);

    // Enumerate the denoms, compute underlying debt and collateral amount, and query the prices.
    // Finally, collect the results into a hashmap indexed by the denoms.
    denoms
        .into_iter()
        .map(|denom| {
            let market = MARKETS.load(deps.storage, &denom)?;
            let params = query_asset_params(&deps.querier, params_addr, &denom)?;

            let collateral_amount = match COLLATERALS.may_load(deps.storage, (user_addr, &denom))? {
                Some(collateral) if collateral.enabled => {
                    let amount_scaled = collateral.amount_scaled;
                    get_underlying_liquidity_amount(amount_scaled, &market, block_time)?
                }
                _ => Uint128::zero(),
            };

            let (debt_amount, uncollateralized_debt) =
                match DEBTS.may_load(deps.storage, (user_addr, &denom))? {
                    Some(debt) => {
                        let debt_amount =
                            get_underlying_debt_amount(debt.amount_scaled, &market, block_time)?;
                        (debt_amount, debt.uncollateralized)
                    }
                    None => (Uint128::zero(), false),
                };

            let asset_price = oracle::helpers::query_price(&deps.querier, oracle_addr, &denom)?;

            let position = Position {
                denom: denom.clone(),
                collateral_amount,
                debt_amount,
                uncollateralized_debt,
                max_ltv: params.max_loan_to_value,
                liquidation_threshold: params.liquidation_threshold,
                asset_price,
            };

            Ok((denom, position))
        })
        .collect()
}
