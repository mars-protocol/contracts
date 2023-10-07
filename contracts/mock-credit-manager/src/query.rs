use cosmwasm_std::{Deps, StdResult};
use mars_rover::msg::query::{ConfigResponse, Positions};
use mars_rover_health_types::AccountKind;

use crate::state::{ACCOUNT_KINDS, CONFIG, POSITION_RESPONSES};

pub fn query_positions(deps: Deps, account_id: String) -> StdResult<Positions> {
    POSITION_RESPONSES.load(deps.storage, &account_id)
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    CONFIG.load(deps.storage)
}

pub fn query_account_kind(deps: Deps, account_id: String) -> StdResult<AccountKind> {
    Ok(ACCOUNT_KINDS.may_load(deps.storage, &account_id)?.unwrap_or(AccountKind::Default))
}
