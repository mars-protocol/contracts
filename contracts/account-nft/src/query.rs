use crate::config::UncheckedConfig;
use crate::state::{CONFIG, NEXT_ID};
use cosmwasm_std::{Deps, StdResult};

pub fn query_config(deps: Deps) -> StdResult<UncheckedConfig> {
    Ok(CONFIG.load(deps.storage)?.into())
}

pub fn query_next_id(deps: Deps) -> StdResult<u64> {
    NEXT_ID.load(deps.storage)
}
