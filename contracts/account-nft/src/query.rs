use cosmwasm_std::{Deps, StdResult};
use mars_account_nft_types::nft_config::UncheckedNftConfig;

use crate::state::{CONFIG, NEXT_ID};

pub fn query_config(deps: Deps) -> StdResult<UncheckedNftConfig> {
    Ok(CONFIG.load(deps.storage)?.into())
}

pub fn query_next_id(deps: Deps) -> StdResult<String> {
    Ok(NEXT_ID.load(deps.storage)?.to_string())
}
