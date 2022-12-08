use std::ops::{Add, Div};

use cosmwasm_std::{Coin, CosmosMsg, Decimal, DepsMut, Env, Response, StdError, Storage, Uint128};

use mars_rover::error::{ContractError, ContractResult};
use mars_rover::msg::execute::CallbackMsg;
use mars_rover::traits::{IntoDecimal, IntoUint128};

use crate::health::{compute_health, val_or_na};
use crate::repay::current_debt_for_denom;
use crate::state::{COIN_BALANCES, MAX_CLOSE_FACTOR, ORACLE, RED_BANK};
use crate::utils::{decrement_coin_balance, increment_coin_balance};

pub fn liquidate_coin(
    deps: DepsMut,
    env: Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt_coin: Coin,
    request_coin_denom: &str,
) -> ContractResult<Response> {
    let request_coin_balance = COIN_BALANCES
        .load(deps.storage, (liquidatee_account_id, request_coin_denom))
        .map_err(|_| ContractError::CoinNotAvailable(request_coin_denom.to_string()))?;

    let (debt, request) = calculate_liquidation(
        &deps,
        &env,
        liquidatee_account_id,
        &debt_coin,
        request_coin_denom,
        request_coin_balance,
    )?;

    let repay_msg = repay_debt(
        deps.storage,
        &env,
        liquidator_account_id,
        liquidatee_account_id,
        &debt,
    )?;

    // Transfer requested coin from liquidatee to liquidator
    decrement_coin_balance(deps.storage, liquidatee_account_id, &request)?;
    increment_coin_balance(deps.storage, liquidator_account_id, &request)?;

    Ok(Response::new()
        .add_message(repay_msg)
        .add_attribute("action", "rover/credit-manager/liquidate_coin")
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
/// Returns -> (Debt Coin, Request Coin)
pub fn calculate_liquidation(
    deps: &DepsMut,
    env: &Env,
    liquidatee_account_id: &str,
    debt_coin: &Coin,
    request_coin: &str,
    request_coin_balance: Uint128,
) -> ContractResult<(Coin, Coin)> {
    // Assert the liquidatee's credit account is liquidatable
    let health = compute_health(deps.as_ref(), env, liquidatee_account_id)?;
    if !health.is_liquidatable() {
        return Err(ContractError::NotLiquidatable {
            account_id: liquidatee_account_id.to_string(),
            lqdt_health_factor: val_or_na(health.liquidation_health_factor),
        });
    }

    // Ensure debt repaid does not exceed liquidatee's total debt for denom
    let (total_debt_amount, _) =
        current_debt_for_denom(deps.as_ref(), env, liquidatee_account_id, &debt_coin.denom)?;

    // Ensure debt amount does not exceed close factor % of the liquidatee's total debt value
    let close_factor = MAX_CLOSE_FACTOR.load(deps.storage)?;
    let max_close_value = close_factor.checked_mul(health.total_debt_value)?;
    let oracle = ORACLE.load(deps.storage)?;
    let debt_res = oracle.query_price(&deps.querier, &debt_coin.denom)?;
    let max_close_amount = max_close_value.div(debt_res.price).uint128();

    // Calculate the maximum debt possible to repay given liquidatee's request coin balance
    // FORMULA: debt amount = request value / (1 + liquidation bonus %) / debt price
    let request_res = oracle.query_price(&deps.querier, request_coin)?;
    let max_request_value = request_res
        .price
        .checked_mul(request_coin_balance.to_dec()?)?;

    let liq_bonus_rate = RED_BANK
        .load(deps.storage)?
        .query_market(&deps.querier, &debt_coin.denom)?
        .liquidation_bonus;
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
    let request_amount = liq_bonus_rate
        .add(Decimal::one())
        .checked_mul(debt_res.price.checked_mul(final_debt_to_repay.to_dec()?)?)?
        .div(request_res.price)
        // Given the nature of integers, these operations will round down. This means the liquidation balance will get
        // closer and closer to 0, but never actually get there and stay as a single denom unit.
        // The remediation for this is to round up at the very end of the calculation.
        .ceil()
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

pub fn repay_debt(
    storage: &mut dyn Storage,
    env: &Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt: &Coin,
) -> ContractResult<CosmosMsg> {
    // Transfer debt coin from liquidator's coin balance to liquidatee
    // Will be used to pay off the debt via CallbackMsg::Repay {}
    decrement_coin_balance(storage, liquidator_account_id, debt)?;
    increment_coin_balance(storage, liquidatee_account_id, debt)?;
    let msg = (CallbackMsg::Repay {
        account_id: liquidatee_account_id.to_string(),
        denom: debt.denom.clone(),
        amount: Some(debt.amount),
    })
    .into_cosmos_msg(&env.contract.address)?;
    Ok(msg)
}
