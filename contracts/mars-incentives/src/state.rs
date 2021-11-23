use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

use mars_core::math::decimal::Decimal;

use crate::{AssetIncentive, Config};

// keys (for singleton)
pub const CONFIG: Item<Config> = Item::new("config");

// namespaces (for buckets)
pub const ASSET_INCENTIVES: Map<&Addr, AssetIncentive> = Map::new("asset_incentives");
pub const USER_ASSET_INDICES: Map<(&Addr, &Addr), Decimal> = Map::new("user_asset_indices");
pub const USER_UNCLAIMED_REWARDS: Map<&Addr, Uint128> = Map::new("user_unclaimed_rewards");
