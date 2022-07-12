use crate::state::PENDING_OWNER;
use cosmwasm_std::{Deps, StdResult};

pub fn query_proposed_new_owner(deps: Deps) -> StdResult<String> {
    Ok(PENDING_OWNER.load(deps.storage)?.into())
}
