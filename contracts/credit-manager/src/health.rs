use std::ops::{Add, Div, Mul};

use cosmwasm_std::{Decimal, Deps, Env, Event, Response, StdResult};

use mock_red_bank::msg::{Market, QueryMsg};
use rover::error::{ContractError, ContractResult};
use rover::health::Health;
use rover::NftTokenId;

use crate::query::query_position;
use crate::state::RED_BANK;

/// Compute the health of a token's position
/// max_tvl = maximum loan to value
/// lqdt = liquidation threshold
pub fn compute_health(deps: Deps, env: &Env, token_id: NftTokenId) -> ContractResult<Health> {
    let position = query_position(deps, env, token_id)?;
    let red_bank = RED_BANK.load(deps.storage)?;

    // The sum of the position's coin asset values w/ loan-to-value adjusted & liquidation threshold adjusted
    let (total_assets_value, max_ltv_adjusted_value, lqdt_adjusted_value) =
        position.coins.iter().try_fold::<_, _, StdResult<_>>(
            (Decimal::zero(), Decimal::zero(), Decimal::zero()),
            |(total, max_ltv_adjusted_total, lqdt_adjusted_total), item| {
                let market: Market = deps.querier.query_wasm_smart(
                    red_bank.address().clone(),
                    &QueryMsg::Market {
                        denom: item.denom.clone(),
                    },
                )?;
                Ok((
                    total.add(item.value),
                    max_ltv_adjusted_total.add(item.value.mul(market.max_loan_to_value)),
                    lqdt_adjusted_total.add(item.value.mul(market.liquidation_threshold)),
                ))
            },
        )?;

    // The sum of the position's debt share values
    let total_debts_value = position
        .debt_shares
        .iter()
        .fold(Decimal::zero(), |total_value, item| {
            total_value.add(item.total_value)
        });

    // Health Factor = Sum(Value of Asset * Liquidation Threshold or Max LTV) / Sum (Value of Total Borrowed)
    // If there aren't any debts a health factor can't be computed (divide by zero)
    let (lqdt_health_factor, max_ltv_health_factor) = if total_debts_value.is_zero() {
        (None, None)
    } else {
        (
            Some(lqdt_adjusted_value.div(total_debts_value)),
            Some(max_ltv_adjusted_value.div(total_debts_value)),
        )
    };

    let liquidatable = lqdt_health_factor.map_or(false, |hf| hf <= Decimal::one());
    let above_max_ltv = max_ltv_health_factor.map_or(false, |hf| hf <= Decimal::one());

    Ok(Health {
        total_assets_value,
        total_debts_value,
        lqdt_health_factor,
        liquidatable,
        max_ltv_health_factor,
        above_max_ltv,
    })
}

pub fn assert_below_max_ltv(
    deps: Deps,
    env: Env,
    token_id: NftTokenId,
) -> ContractResult<Response> {
    let position = compute_health(deps, &env, token_id)?;

    if position.above_max_ltv {
        return Err(ContractError::AboveMaxLTV);
    }

    let event = Event::new("position_changed")
        .add_attribute("timestamp", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("token_id", token_id)
        .add_attribute("assets_value", position.total_assets_value.to_string())
        .add_attribute("debts_value", position.total_debts_value.to_string())
        .add_attribute(
            "lqdt_health_factor",
            val_or_not_applicable(position.lqdt_health_factor),
        )
        .add_attribute("liquidatable", position.liquidatable.to_string())
        .add_attribute(
            "max_ltv_health_factor",
            val_or_not_applicable(position.max_ltv_health_factor),
        )
        .add_attribute("above_max_ltv", position.above_max_ltv.to_string());

    Ok(Response::new()
        .add_attribute("action", "rover/credit_manager/callback/assert_health")
        .add_event(event))
}

fn val_or_not_applicable(opt: Option<Decimal>) -> String {
    opt.map_or_else(|| "n/a".to_string(), |dec| dec.to_string())
}
