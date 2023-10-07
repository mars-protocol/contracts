use cosmwasm_std::{Deps, Response};
use mars_red_bank_types::oracle::ActionKind;
use mars_rover::error::{ContractError, ContractResult};
use mars_rover_health_types::{HealthState, HealthValuesResponse};

use crate::{state::HEALTH_CONTRACT, utils::get_account_kind};

pub fn query_health_state(
    deps: Deps,
    account_id: &str,
    action: ActionKind,
) -> ContractResult<HealthState> {
    let hc = HEALTH_CONTRACT.load(deps.storage)?;
    let kind = get_account_kind(deps.storage, account_id)?;
    Ok(hc.query_health_state(&deps.querier, account_id, kind, action)?)
}

pub fn query_health_values(
    deps: Deps,
    account_id: &str,
    action: ActionKind,
) -> ContractResult<HealthValuesResponse> {
    let hc = HEALTH_CONTRACT.load(deps.storage)?;
    let kind = get_account_kind(deps.storage, account_id)?;
    Ok(hc.query_health_values(&deps.querier, account_id, kind, action)?)
}

pub fn assert_max_ltv(
    deps: Deps,
    account_id: &str,
    prev_health: HealthState,
) -> ContractResult<Response> {
    let new_health = query_health_state(deps, account_id, ActionKind::Default)?;

    match (&prev_health, &new_health) {
        // If account ends in a healthy state, all good! ✅
        (_, HealthState::Healthy) => {}
        // If previous health was in an unhealthy state, assert it did not further weaken ⚠️
        (
            HealthState::Unhealthy {
                max_ltv_health_factor: prev_hf,
                ..
            },
            HealthState::Unhealthy {
                max_ltv_health_factor: new_hf,
                ..
            },
        ) => {
            if prev_hf > new_hf {
                return Err(ContractError::HealthNotImproved {
                    prev_hf: prev_hf.to_string(),
                    new_hf: new_hf.to_string(),
                });
            }
        }
        // Else, it went from healthy to unhealthy, raise! ❌
        (
            HealthState::Healthy,
            HealthState::Unhealthy {
                max_ltv_health_factor,
                ..
            },
        ) => {
            return Err(ContractError::AboveMaxLTV {
                account_id: account_id.to_string(),
                max_ltv_health_factor: max_ltv_health_factor.to_string(),
            });
        }
    }

    Ok(Response::new()
        .add_attribute("action", "callback/assert_health")
        .add_attribute("account_id", account_id)
        .add_attribute("prev_health_state", prev_health.to_string())
        .add_attribute("new_health_state", new_health.to_string()))
}
