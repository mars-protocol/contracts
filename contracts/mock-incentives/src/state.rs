use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::Map;

// Map<(Addr, CmAccountId), Unclaimed Coins>
pub const UNCLAIMED_REWARDS: Map<(Addr, String), Vec<Coin>> = Map::new("unclaimed_rewards");
