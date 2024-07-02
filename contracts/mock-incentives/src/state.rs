use cosmwasm_std::{Addr, Coin, Uint128};
use cw_storage_plus::Map;
pub const DEFAULT_LIMIT: u32 = 10;
pub const MAX_LIMIT: u32 = 30;

// Map<(Addr, CmAccountId), Unclaimed Coins>
pub const UNCLAIMED_REWARDS: Map<(Addr, String), Vec<Coin>> = Map::new("unclaimed_rewards");

// Map<(account_id, lp_denom), PendingRewards>
pub const PENDING_ASTRO_REWARDS: Map<(String, String), Vec<Coin>> =
    Map::new("pending_astro_rewards");

// Map<(account_id, lp_denom), staked amount>
pub const STAKED_ASTRO_LP_POSITIONS: Map<(String, String), Uint128> =
    Map::new("staked_astro_lp_positions");
