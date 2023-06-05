use cosmwasm_std::{Deps, StdResult};
use mars_rover::msg::query::{ConfigResponse, Positions};

use crate::state::{CONFIG, POSITION_RESPONSES};

pub fn query_positions(deps: Deps, account_id: String) -> StdResult<Positions> {
    POSITION_RESPONSES.load(deps.storage, &account_id)
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    CONFIG.load(deps.storage)
}
