use std::ops::{Add, Div};

use cosmwasm_std::{Coin, Decimal, Deps, DepsMut, Env, Response, StdError};
use mars_health::health::Health;

use rover::error::{ContractError, ContractResult};
use rover::msg::execute::CallbackMsg;

use crate::health::{compute_health, val_or_na};
use crate::repay::current_debt_for_denom;
use crate::state::{COIN_BALANCES, MAX_CLOSE_FACTOR, MAX_LIQUIDATION_BONUS, ORACLE};
use crate::utils::{decrement_coin_balance, increment_coin_balance, IntoUint128};

pub fn liquidate_coin(
    deps: DepsMut,
    env: Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt_coin: Coin,
    request_coin_denom: &str,
) -> ContractResult<Response> {
    // Assert the liquidatee's credit account is liquidatable
    let health = compute_health(deps.as_ref(), &env, liquidatee_account_id)?;
    if !health.is_liquidatable() {
        return Err(ContractError::NotLiquidatable {
            account_id: liquidatee_account_id.to_string(),
            lqdt_health_factor: val_or_na(health.liquidation_health_factor),
        });
    }

    let (debt, request) = calculate_liquidation_request(
        &deps,
        &env,
        liquidatee_account_id,
        &debt_coin,
        request_coin_denom,
        &health,
    )?;

    // Transfer debt coin from liquidator's coin balance to liquidatee
    // Will be used to pay off the debt via CallbackMsg::Repay {}
    decrement_coin_balance(deps.storage, liquidator_account_id, &debt)?;
    increment_coin_balance(deps.storage, liquidatee_account_id, &debt)?;
    let repay_msg = (CallbackMsg::Repay {
        account_id: liquidatee_account_id.to_string(),
        coin: debt.clone(),
    })
    .into_cosmos_msg(&env.contract.address)?;

    // Transfer requested coin from liquidatee to liquidator
    decrement_coin_balance(deps.storage, liquidatee_account_id, &request)?;
    increment_coin_balance(deps.storage, liquidator_account_id, &request)?;

    // Ensure health factor has improved as a consequence of liquidation event
    let assert_healthier_msg = (CallbackMsg::AssertHealthFactorImproved {
        account_id: liquidatee_account_id.to_string(),
        previous_health_factor: health.liquidation_health_factor.unwrap(), // safe unwrap given it was liquidatable
    })
    .into_cosmos_msg(&env.contract.address)?;

    Ok(Response::new()
        .add_message(repay_msg)
        .add_message(assert_healthier_msg)
        .add_attribute("action", "rover/credit_manager/liquidate")
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("debt_repaid_denom", debt.denom)
        .add_attribute("debt_repaid_amount", debt.amount)
        .add_attribute("request_coin_denom", request.denom)
        .add_attribute("request_coin_amount", request.amount))
}

/// Calculates precise debt & request coin amounts to liquidate
/// The debt amount will be adjusted down if:
/// - Exceeds liquidatee's total debt for denom
/// - Not enough liquidatee request coin balance to match
/// - The value of the debt repaid exceeds the maximum close factor %
fn calculate_liquidation_request(
    deps: &DepsMut,
    env: &Env,
    liquidatee_account_id: &str,
    debt_coin: &Coin,
    request_coin: &str,
    health: &Health,
) -> ContractResult<(Coin, Coin)> {
    // Ensure debt repaid does not exceed liquidatee's total debt for denom
    let (total_debt_amount, _) =
        current_debt_for_denom(deps.as_ref(), env, liquidatee_account_id, debt_coin)?;

    // Ensure debt amount does not exceed close factor % of the liquidatee's total debt value
    let close_factor = MAX_CLOSE_FACTOR.load(deps.storage)?;
    let max_close_value = close_factor.checked_mul(health.total_debt_value)?;
    let oracle = ORACLE.load(deps.storage)?;
    let debt_res = oracle.query_price(&deps.querier, &debt_coin.denom)?;
    let max_close_amount = max_close_value.div(debt_res.price).uint128();

    // Calculate the maximum debt possible to repay given liquidatee's request coin balance
    // FORMULA: debt amount = request value / (1 + liquidation bonus %) / debt price
    let liquidatee_balance = COIN_BALANCES
        .load(deps.storage, (liquidatee_account_id, request_coin))
        .map_err(|_| ContractError::CoinNotAvailable(request_coin.to_string()))?;
    let request_res = oracle.query_price(&deps.querier, request_coin)?;
    let balance_dec = Decimal::from_atomics(liquidatee_balance, 0)?;
    let max_request_value = request_res.price.checked_mul(balance_dec)?;
    let liq_bonus_rate = MAX_LIQUIDATION_BONUS.load(deps.storage)?;
    let request_coin_adjusted_max_debt = max_request_value
        .div(liq_bonus_rate.add(Decimal::one()))
        .div(debt_res.price)
        .uint128();

    let final_debt_to_repay = *vec![
        debt_coin.amount,
        total_debt_amount,
        max_close_amount,
        request_coin_adjusted_max_debt,
    ]
    .iter()
    .min()
    .ok_or_else(|| StdError::generic_err("Minimum not found"))?;

    // Calculate exact request coin amount to give to liquidator
    // FORMULA: request amount = (1 + liquidation bonus %) * debt value / request coin price
    let debt_amount_dec = Decimal::from_atomics(final_debt_to_repay, 0)?;
    let request_amount = liq_bonus_rate
        .add(Decimal::one())
        .checked_mul(debt_res.price.checked_mul(debt_amount_dec)?)?
        .div(request_res.price)
        .uint128();

    // (Debt Coin, Request Coin)
    Ok((
        Coin {
            denom: debt_coin.denom.clone(),
            amount: final_debt_to_repay,
        },
        Coin {
            denom: request_coin.to_string(),
            amount: request_amount,
        },
    ))
}

pub fn assert_health_factor_improved(
    deps: Deps,
    env: Env,
    account_id: &str,
    prev_hf: Decimal,
) -> ContractResult<Response> {
    let health = compute_health(deps, &env, account_id)?;
    if let Some(hf) = health.liquidation_health_factor {
        if prev_hf > hf {
            return Err(ContractError::HealthNotImproved {
                prev_hf: prev_hf.to_string(),
                new_hf: hf.to_string(),
            });
        }
    }
    Ok(Response::new()
        .add_attribute(
            "action",
            "rover/credit_manager/assert_health_factor_improved",
        )
        .add_attribute("prev_hf", prev_hf.to_string())
        .add_attribute("new_hf", val_or_na(health.liquidation_health_factor)))
}
