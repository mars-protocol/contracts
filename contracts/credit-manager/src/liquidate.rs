use cosmwasm_std::{Coin, DepsMut, QuerierWrapper, Uint128};
use mars_liquidation::liquidation::calculate_liquidation_amounts;
use mars_red_bank_types::oracle::ActionKind;
use mars_rover::{
    adapters::oracle::Oracle,
    error::{ContractError, ContractResult},
    traits::Stringify,
};

use crate::{
    health::query_health_values,
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
    liquidatee_account_id: &str,
    debt_coin: &Coin,
    request_coin: &str,
    request_coin_balance: Uint128,
) -> ContractResult<(Coin, Coin, Coin)> {
    // Assert the liquidatee's credit account is liquidatable
    let health =
        query_health_values(deps.as_ref(), liquidatee_account_id, ActionKind::Liquidation)?;
    if !health.liquidatable {
        return Err(ContractError::NotLiquidatable {
            account_id: liquidatee_account_id.to_string(),
            lqdt_health_factor: health.liquidation_health_factor.to_string(),
        });
    }

    // Ensure debt repaid does not exceed liquidatee's total debt for denom
    let (total_debt_amount, _) =
        current_debt_for_denom(deps.as_ref(), liquidatee_account_id, &debt_coin.denom)?;

    let params = PARAMS.load(deps.storage)?;
    let target_health_factor = params.query_target_health_factor(&deps.querier)?;
    let request_coin_params = params.query_asset_params(&deps.querier, request_coin)?;

    let oracle = ORACLE.load(deps.storage)?;
    let debt_coin_price =
        oracle.query_price(&deps.querier, &debt_coin.denom, ActionKind::Liquidation)?.price;
    let request_coin_price =
        oracle.query_price(&deps.querier, request_coin, ActionKind::Liquidation)?.price;

    let (debt_amount_to_repay, request_amount_to_liquidate, request_amount_received_by_liquidator) =
        calculate_liquidation_amounts(
            request_coin_balance,
            request_coin_price,
            &request_coin_params,
            total_debt_amount,
            debt_coin.amount,
            debt_coin_price,
            target_health_factor,
            &health.into(),
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

/// In scenarios with small amounts or large gap between coin prices, there is a possibility
/// that the liquidation will result in loss for the liquidator. This assertion prevents this.
fn assert_liquidation_profitable(
    querier: &QuerierWrapper,
    oracle: &Oracle,
    (debt_coin, request_coin, ..): (Coin, Coin, Coin),
) -> ContractResult<()> {
    let debt_value = oracle.query_value(querier, &debt_coin, ActionKind::Liquidation)?;
    let request_value = oracle.query_value(querier, &request_coin, ActionKind::Liquidation)?;

    if debt_value >= request_value {
        return Err(ContractError::LiquidationNotProfitable {
            debt_coin,
            request_coin,
        });
    }

    Ok(())
}

/// Guards against the case an account is trying to liquidate itself
pub fn assert_not_self_liquidation(
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
) -> ContractResult<()> {
    if liquidator_account_id == liquidatee_account_id {
        return Err(ContractError::SelfLiquidation);
    }
    Ok(())
}
