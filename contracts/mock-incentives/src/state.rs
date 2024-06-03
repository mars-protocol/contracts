use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::Map;

// Map<(Addr, CmAccountId), Unclaimed Coins>
pub const UNCLAIMED_REWARDS: Map<(Addr, String), Vec<Coin>> = Map::new("unclaimed_rewards");

// Map<(account_id, lp_denom), PendingRewards>
pub const PENDING_ASTROPORT_REWARDS: Map<(String, String), Vec<Coin>> =
    Map::new("pending_astroport_rewards");
