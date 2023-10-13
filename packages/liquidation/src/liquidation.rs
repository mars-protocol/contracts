use std::{
    cmp::{max, min},
    ops::Add,
};

use cosmwasm_std::{Decimal, StdError, Uint128};
use mars_health::health::Health;
use mars_types::params::AssetParams;

use crate::error::LiquidationError;

/// Within this new system, the close factor (CF) will be determined dynamically using a parameter
/// known as the Target Health Factor (THF). The THF determines the ideal HF a position should be left
/// at immediately after the position has been liquidated. The CF, in turn, is a result of this parameter:
/// the maximum amount of debt that can be repaid to take the position to the THF.
/// For example, if the THF is 1.10 and a position gets liquidated at HF = 0.98, then the maximum
/// amount of debt a liquidator can repay (in other words, the CF) will be an amount such that the HF
/// after the liquidation is at maximum 1.10.
///
/// The formula to calculate the maximum debt that can be repaid by a liquidator is as follows:
/// MDR_value = (THF * total_debt_value - liq_th_collateral_value) / (THF - (requested_collateral_liq_th * (1 + LB)))
/// where:
/// MDR                         - Maximum Debt Repayable
/// THF                         - Target Health Factor
/// total_debt_value            - Value of debt before the liquidation happens
/// liq_th_collateral_value     - Value of collateral before the liquidation happens adjusted to liquidation threshold
/// requested_collateral_liq_th - Liquidation threshold of requested collateral
/// LB                          - Liquidation Bonus
///
/// PLF (Protocol Liqudiation Fee) is charged as a % of the LB.
/// For example, if we define the PLF as 10%, then the PLF would be deducted from the LB, so upon a liquidation:
/// - The liquidator receives 90% of the LB.
/// - The remaining 10% is sent to the protocol as PLF.
#[allow(clippy::too_many_arguments)]
pub fn calculate_liquidation_amounts(
    collateral_amount: Uint128,
    collateral_price: Decimal,
    collateral_params: &AssetParams,
    debt_amount: Uint128,
    debt_requested_to_repay: Uint128,
    debt_price: Decimal,
    target_health_factor: Decimal,
    health: &Health,
) -> Result<(Uint128, Uint128, Uint128), LiquidationError> {
    // if health.liquidatable == true, save to unwrap
    let liquidation_health_factor = health.liquidation_health_factor.unwrap();

    let user_collateral_value = collateral_amount.checked_mul_floor(collateral_price)?;

    let liquidation_bonus = calculate_liquidation_bonus(
        liquidation_health_factor,
        health.total_collateral_value,
        health.total_debt_value,
        collateral_params,
    )?;

    // All debt is liquidatable: When MDR < 0, it means even repaying the whole debt is not going to be enough
    // to bring the account back to the THF, so the liquidator should be able to repay all the available debt.
    // Given the numerator in the MDR formula is always > 0, MDR < 0 happens when the denominator is < 0
    // (we include the case where it’s 0 given it would make MDR = infinite).
    let formula = collateral_params.liquidation_threshold * (Decimal::one() + liquidation_bonus);
    let max_debt_repayable_amount = if formula < target_health_factor {
        let max_debt_repayable_numerator = (target_health_factor * health.total_debt_value)
            - health.liquidation_threshold_adjusted_collateral;

        let max_debt_repayable_denominator = target_health_factor - formula;

        let max_debt_repayable_value =
            max_debt_repayable_numerator.checked_div_floor(max_debt_repayable_denominator)?;

        let max_debt_repayable_amount = max_debt_repayable_value.checked_div_floor(debt_price)?;
        Some(max_debt_repayable_amount)
    } else {
        None
    };

    // calculate possible debt to repay based on available collateral
    let debt_amount_possible_to_repay = user_collateral_value
        .checked_div_floor(Decimal::one().add(liquidation_bonus))?
        .checked_div_floor(debt_price)?;

    let debt_amount_to_repay = *[
        Some(debt_amount),
        Some(debt_requested_to_repay),
        max_debt_repayable_amount,
        Some(debt_amount_possible_to_repay),
    ]
    .iter()
    .flatten()
    .min()
    .ok_or_else(|| StdError::generic_err("Minimum not found"))?;

    let debt_value_to_repay = debt_amount_to_repay.checked_mul_floor(debt_price)?;

    let collateral_amount_to_liquidate = debt_value_to_repay
        .checked_mul_floor(liquidation_bonus.add(Decimal::one()))?
        .checked_div_floor(collateral_price)?;

    // In some edges scenarios:
    // - if debt_amount_to_repay = 0, some liquidators could drain collaterals and all their coins
    // would be refunded, i.e.: without spending coins.
    // - if collateral_amount_to_liquidate is 0, some users could liquidate without receiving collaterals
    // in return.
    if (!collateral_amount_to_liquidate.is_zero() && debt_amount_to_repay.is_zero())
        || (collateral_amount_to_liquidate.is_zero() && !debt_amount_to_repay.is_zero())
    {
        return Err(LiquidationError::Std(StdError::generic_err(
            format!("Can't process liquidation. Invalid collateral_amount_to_liquidate ({collateral_amount_to_liquidate}) and debt_amount_to_repay ({debt_amount_to_repay})")
        )));
    }

    let lb_value = debt_value_to_repay.checked_mul_floor(liquidation_bonus)?;

    // Use ceiling in favour of protocol
    let protocol_fee_value =
        lb_value.checked_mul_ceil(collateral_params.protocol_liquidation_fee)?;
    let protocol_fee_amount = protocol_fee_value.checked_div_floor(collateral_price)?;

    let collateral_amount_received_by_liquidator =
        collateral_amount_to_liquidate - protocol_fee_amount;

    Ok((
        debt_amount_to_repay,
        collateral_amount_to_liquidate,
        collateral_amount_received_by_liquidator,
    ))
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
) -> Result<Decimal, LiquidationError> {
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
