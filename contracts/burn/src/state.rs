use cosmwasm_std::Uint128;
use cw_storage_plus::Map;

pub const BURNT_AMOUNTS: Map<&str, Uint128> = Map::new("burnt_amounts");
