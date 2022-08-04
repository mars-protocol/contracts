use std::ops::{Add, Div, Mul};

use cosmwasm_std::{Decimal, Deps, DepsMut, Env, Event, Response, StdResult, Uint128};

use mock_red_bank::msg::{Market, QueryMsg};
use rover::error::{ContractError, ContractResult};
use rover::health::Health;
use rover::msg::query::{CoinSharesValue, CoinValue};
use rover::NftTokenId;

use crate::query::query_position;
use crate::state::RED_BANK;

/// Compute the health of a token's position
pub fn compute_health(
    deps: &Deps,
    assets: &[CoinValue],
    debts: &[CoinSharesValue],
) -> ContractResult<Health> {
    // The sum of the position's assets weighted by max LTV
    let (ltv_adjusted_assets_value, assets_value) = assets.iter().try_fold::<_, _, StdResult<_>>(
        (Decimal::zero(), Decimal::zero()),
        |(ltv_adjusted_total, base_total), item| {
            let red_bank = RED_BANK.load(deps.storage)?;
            let market: Market = deps.querier.query_wasm_smart(
                red_bank.0,
                &QueryMsg::Market {
                    denom: item.denom.clone(),
                },
            )?;
            Ok((
                ltv_adjusted_total.add(item.value.mul(market.max_loan_to_value)),
                base_total.add(item.value),
            ))
        },
    )?;

    let (debts_shares, debts_value) = debts.iter().fold(
        (Uint128::zero(), Decimal::zero()),
        |(total_shares, total_value), item| {
            (total_shares.add(item.shares), total_value.add(item.value))
        },
    );

    // Health Factor = Sum(Value of Asset * Max LTV) / Sum (Value of Total Borrowed)
    // If there aren't any debts a health factor can't be computed (divide by zero)
    let health_factor = if debts_value.is_zero() {
        None
    } else {
        Some(ltv_adjusted_assets_value.div(debts_value))
    };

    // If Some(health_factor), we assert it is no less than 1
    // If it is None, meaning `debts_value` is zero, we assert debt shares are also zero
    //
    // NOTE: We assert debt shares are zero, instead of `debts_value`.
    // This is because value can be zero as a result of rounding down.
    let healthy = if let Some(hf) = health_factor {
        hf > Decimal::one()
    } else {
        debts_shares.is_zero()
    };

    Ok(Health {
        assets_value,
        ltv_adjusted_assets_value,
        debts_value,
        health_factor,
        healthy,
    })
}

pub fn assert_health(deps: DepsMut, env: Env, token_id: NftTokenId) -> ContractResult<Response> {
    let position = query_position(deps.as_ref(), &env, token_id)?;
    let hf_str = position
        .health_factor
        .map_or("n/a".to_string(), |dec| dec.to_string());

    if !position.healthy {
        return Err(ContractError::AccountUnhealthy {
            health_factor: hf_str,
        });
    }

    let event = Event::new("position_changed")
        .add_attribute("timestamp", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("token_id", token_id)
        .add_attribute("assets_value", position.assets_value.to_string())
        .add_attribute("debt_value", position.debts_value.to_string())
        .add_attribute("health_factor", hf_str)
        .add_attribute("healthy", position.healthy.to_string());

    Ok(Response::new()
        .add_attribute("action", "rover/credit_manager/callback/assert_health")
        .add_event(event))
}
