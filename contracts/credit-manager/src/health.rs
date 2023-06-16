use cosmwasm_std::{Decimal, Deps, Env, Event, Response};
use mars_rover::{
    error::{ContractError, ContractResult},
    traits::Stringify,
};
use mars_rover_health_types::{is_below_one, HealthResponse};

use crate::{state::HEALTH_CONTRACT, utils::get_account_kind};

pub fn query_health(deps: Deps, account_id: &str) -> ContractResult<HealthResponse> {
    let hc = HEALTH_CONTRACT.load(deps.storage)?;
    let kind = get_account_kind(deps.storage, account_id)?;
    Ok(hc.query_health(&deps.querier, account_id, kind)?)
}

pub fn assert_max_ltv(
    deps: Deps,
    env: Env,
    account_id: &str,
    prev_max_ltv_health_factor: &Option<Decimal>,
) -> ContractResult<Response> {
    let new_health = query_health(deps, account_id)?;

    // If previous health was in a bad state, assert it did not further weaken
    if is_below_one(prev_max_ltv_health_factor) {
        if let (Some(prev_hf), Some(new_hf)) =
            (prev_max_ltv_health_factor, new_health.max_ltv_health_factor)
        {
            if prev_hf > &new_hf {
                return Err(ContractError::HealthNotImproved {
                    prev_hf: prev_hf.to_string(),
                    new_hf: new_hf.to_string(),
                });
            }
        }
    // if previous health was in a good state, assert it's still healthy
    } else if new_health.above_max_ltv {
        return Err(ContractError::AboveMaxLTV {
            account_id: account_id.to_string(),
            max_ltv_health_factor: new_health.max_ltv_health_factor.to_string(),
        });
    }

    let event = Event::new("position_changed")
        .add_attribute("timestamp", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("account_id", account_id)
        .add_attribute("collateral_value", new_health.total_collateral_value.to_string())
        .add_attribute("debts_value", new_health.total_debt_value.to_string())
        .add_attribute("lqdt_health_factor", new_health.liquidation_health_factor.to_string())
        .add_attribute("liquidatable", new_health.liquidatable.to_string())
        .add_attribute("max_ltv_health_factor", new_health.max_ltv_health_factor.to_string())
        .add_attribute("above_max_ltv", new_health.above_max_ltv.to_string());

    Ok(Response::new().add_attribute("action", "callback/assert_health").add_event(event))
}
