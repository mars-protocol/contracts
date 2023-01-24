use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};
use mars_owner::Owner;
use mars_types::incentives::{AssetIncentive, Config};

// keys (for singleton)
pub const OWNER: Owner = Owner::new("owner");
pub const CONFIG: Item<Config> = Item::new("config");

// namespaces (for buckets)
pub const ASSET_INCENTIVES: Map<&str, AssetIncentive> = Map::new("incentives");
pub const USER_ASSET_INDICES: Map<(&Addr, &str), Decimal> = Map::new("indices");
pub const USER_UNCLAIMED_REWARDS: Map<&Addr, Uint128> = Map::new("unclaimed_rewards");
