use std::{
    cmp::{max, min},
    ops::Add,
};

use cosmwasm_std::{Coin, Decimal, DepsMut, Env, QuerierWrapper, StdError, Uint128};
use mars_params::types::asset::AssetParams;
use mars_rover::{
    adapters::oracle::Oracle,
    error::{ContractError, ContractResult},
    traits::Stringify,
};
use mars_rover_health_types::HealthResponse;

use crate::{
    health::query_health,
    repay::current_debt_for_denom,
    state::{ORACLE, PARAMS},
};

/// Calculates precise debt, request coin amounts to liquidate, request coin transfered to liquidator and rewards-collector.
/// The debt amount will be adjusted down if:
/// - Exceeds liquidatee's total debt for denom
/// - Not enough liquidatee request coin balance to match
/// - The value of the debt repaid exceeds the Maximum Debt Repayable (MDR)
/// Returns -> (Debt Coin, Liquidator Request Coin, Liquidatee Request Coin)
/// Difference between Liquidator Request Coin and Liquidatee Request Coin goes to rewards-collector account as protocol fee.
pub fn calculate_liquidation(
    deps: &DepsMut,
    env: &Env,
    liquidatee_account_id: &str,
    debt_coin: &Coin,
    request_coin: &str,
    request_coin_balance: Uint128,
) -> ContractResult<(Coin, Coin, Coin)> {
    // Assert the liquidatee's credit account is liquidatable
    let health = query_health(deps.as_ref(), liquidatee_account_id)?;
    if !health.liquidatable {
        return Err(ContractError::NotLiquidatable {
            account_id: liquidatee_account_id.to_string(),
            lqdt_health_factor: health.liquidation_health_factor.to_string(),
        });
    }

    // Ensure debt repaid does not exceed liquidatee's total debt for denom
    let (total_debt_amount, _) =
        current_debt_for_denom(deps.as_ref(), env, liquidatee_account_id, &debt_coin.denom)?;

    let params = PARAMS.load(deps.storage)?;
    let target_health_factor = params.query_target_health_factor(&deps.querier)?;
    let request_coin_params = params.query_asset_params(&deps.querier, request_coin)?;

    let oracle = ORACLE.load(deps.storage)?;
    let debt_coin_price = oracle.query_price(&deps.querier, &debt_coin.denom)?.price;
    let request_coin_price = oracle.query_price(&deps.querier, request_coin)?.price;

    let (debt_amount_to_repay, request_amount_to_liquidate, request_amount_received_by_liquidator) =
        calculate_liquidation_amounts(
            request_coin_balance,
            request_coin_price,
            &request_coin_params,
            total_debt_amount,
            debt_coin.amount,
            debt_coin_price,
            target_health_factor,
            &health,
        )?;

    // (Debt Coin, Liquidator Request Coin, Liquidatee Request Coin)
    let result = (
        Coin {
            denom: debt_coin.denom.clone(),
            amount: debt_amount_to_repay,
        },
        Coin {
            denom: request_coin.to_string(),
            amount: request_amount_received_by_liquidator,
        },
        Coin {
            denom: request_coin.to_string(),
            amount: request_amount_to_liquidate,
        },
    );

    assert_liquidation_profitable(&deps.querier, &oracle, result.clone())?;

    Ok(result)
}

/// Within this new system, the close factor (CF) will be determined dynamically using a parameter
/// known as the Target Health Factor (THF). The THF determines the ideal HF a position should be left
/// at immediately after the position has been liquidated. The CF, in turn, is a result of this parameter:
/// the maximum amount of debt that can be repaid to take the position to the THF.
/// For example, if the THF is 1.10 and a position gets liquidated at HF = 0.98, then the maximum
/// amount of debt a liquidator can repay (in other words, the CF) will be an amount such that the HF
/// after the liquidation is at maximum 1.10.
///
/// The formula to calculate the maximum debt that can be repaid by a liquidator is as follows:
/// MDR_value = (THF * total_debt_value - liq_th_collateral_value) / (THF - (requested_collateral_liq_th * (1 + TLF)))
/// where:
/// MDR                         - Maximum Debt Repayable
/// THF                         - Target Health Factor
/// total_debt_value            - Value of debt before the liquidation happens
/// liq_th_collateral_value     - Value of collateral before the liquidation happens adjusted to liquidation threshold
/// requested_collateral_liq_th - Liquidation threshold of requested collateral
/// TLF                         - Total Liquidation Fee
#[allow(clippy::too_many_arguments)]
fn calculate_liquidation_amounts(
    collateral_amount: Uint128,
    collateral_price: Decimal,
    collateral_params: &AssetParams,
    debt_amount: Uint128,
    debt_requested_to_repay: Uint128,
    debt_price: Decimal,
    target_health_factor: Decimal,
    health: &HealthResponse,
) -> Result<(Uint128, Uint128, Uint128), ContractError> {
    // if health.liquidatable == true, save to unwrap
    let liquidation_health_factor = health.liquidation_health_factor.unwrap();

    let user_collateral_value = collateral_amount.checked_mul_floor(collateral_price)?;

    let liquidation_bonus = calculate_liquidation_bonus(
        liquidation_health_factor,
        health.total_collateral_value,
        health.total_debt_value,
        collateral_params,
    )?;

    let updated_tlf = calculate_total_liquidation_fee(
        liquidation_health_factor,
        liquidation_bonus,
        collateral_params,
    )?;

    let max_debt_repayable_numerator = (target_health_factor * health.total_debt_value)
        - health.liquidation_threshold_adjusted_collateral;
    let max_debt_repayable_denominator = target_health_factor
        - (collateral_params.liquidation_threshold * (Decimal::one() + updated_tlf));

    let max_debt_repayable_value =
        max_debt_repayable_numerator.checked_div_floor(max_debt_repayable_denominator)?;

    let max_debt_repayable_amount = max_debt_repayable_value.checked_div_floor(debt_price)?;

    // calculate possible debt to repay based on available collateral
    let debt_amount_possible_to_repay = user_collateral_value
        .checked_div_floor(Decimal::one().add(updated_tlf))?
        .checked_div_floor(debt_price)?;

    let debt_amount_to_repay = *vec![
        debt_amount,
        debt_requested_to_repay,
        max_debt_repayable_amount,
        debt_amount_possible_to_repay,
    ]
    .iter()
    .min()
    .ok_or_else(|| StdError::generic_err("Minimum not found"))?;

    let collateral_amount_to_liquidate = debt_amount_to_repay
        .checked_mul_floor(debt_price)?
        .checked_mul_floor(updated_tlf.add(Decimal::one()))?
        .checked_div_floor(collateral_price)?;
    let collateral_amount_received_by_liquidator = debt_amount_to_repay
        .checked_mul_floor(debt_price)?
        .checked_mul_floor(liquidation_bonus.add(Decimal::one()))?
        .checked_div_floor(collateral_price)?;

    Ok((
        debt_amount_to_repay,
        collateral_amount_to_liquidate,
        collateral_amount_received_by_liquidator,
    ))
}

/// In order for HF after liquidation to be higher than HF before liquidation, it is necessary that the condition holds:
/// max_total_liquidation_fee <= (liquidation_health_factor / requested_collateral_liq_th) - 1
/// Based on that info we derive max protocol liquidation fee. It is OK to be 0.
///
/// For more info see: https://docs.google.com/document/d/1kImPm4xd3pP8EaC1KZU8oLMFciRDd8Z-jOxv0MTQvEc/edit?usp=sharing
fn calculate_total_liquidation_fee(
    liquidation_health_factor: Decimal,
    liquidation_bonus: Decimal,
    collateral_params: &AssetParams,
) -> Result<Decimal, ContractError> {
    let max_tlf = liquidation_health_factor.checked_div(collateral_params.liquidation_threshold)?;
    let max_tlf = if max_tlf > Decimal::one() {
        max_tlf - Decimal::one()
    } else {
        Decimal::zero()
    };
    let available_plf = if max_tlf > liquidation_bonus {
        max_tlf - liquidation_bonus
    } else {
        Decimal::zero()
    };
    let updated_plf = min(collateral_params.protocol_liquidation_fee, available_plf);
    let updated_tlf = updated_plf + liquidation_bonus;
    Ok(updated_tlf)
}

/// The LB will depend on the Health Factor and a couple other parameters as follows:
/// Liquidation Bonus = min(
///     starting_lb + (slope * (1 - HF)),
///     max(
///         min(CR - 1, max_lb),
///         min_lb
///     )
/// )
/// `CR` is the Collateralization Ratio of the position calculated as `CR = Total Assets / Total Debt`.
fn calculate_liquidation_bonus(
    liquidation_health_factor: Decimal,
    total_collateral_value: Uint128,
    total_debt_value: Uint128,
    collateral_params: &AssetParams,
) -> Result<Decimal, ContractError> {
    let collateralization_ratio =
        Decimal::checked_from_ratio(total_collateral_value, total_debt_value)?;

    // (CR - 1) can't be negative
    let collateralization_ratio_adjusted = if collateralization_ratio > Decimal::one() {
        collateralization_ratio - Decimal::one()
    } else {
        Decimal::zero()
    };

    let max_lb_adjusted = max(
        min(collateralization_ratio_adjusted, collateral_params.liquidation_bonus.max_lb),
        collateral_params.liquidation_bonus.min_lb,
    );

    let calculated_bonus = collateral_params.liquidation_bonus.starting_lb.checked_add(
        collateral_params
            .liquidation_bonus
            .slope
            .checked_mul(Decimal::one() - liquidation_health_factor)?,
    )?;

    let liquidation_bonus = min(calculated_bonus, max_lb_adjusted);

    Ok(liquidation_bonus)
}

/// In scenarios with small amounts or large gap between coin prices, there is a possibility
/// that the liquidation will result in loss for the liquidator. This assertion prevents this.
fn assert_liquidation_profitable(
    querier: &QuerierWrapper,
    oracle: &Oracle,
    (debt_coin, request_coin, ..): (Coin, Coin, Coin),
) -> ContractResult<()> {
    let debt_value = oracle.query_total_value(querier, &[debt_coin.clone()])?;
    let request_value = oracle.query_total_value(querier, &[request_coin.clone()])?;

    if debt_value >= request_value {
        return Err(ContractError::LiquidationNotProfitable {
            debt_coin,
            request_coin,
        });
    }

    Ok(())
}
