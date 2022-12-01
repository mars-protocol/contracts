use crate::config::UncheckedConfig;
use crate::state::CONFIG;
use cosmwasm_std::{Deps, StdResult};

pub fn query_config(deps: Deps) -> StdResult<UncheckedConfig> {
    Ok(CONFIG.load(deps.storage)?.into())
}
