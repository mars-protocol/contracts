use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

use mars_core::vesting::{Allocation, Config, Snapshot};

pub const CONFIG: Item<Config<Addr>> = Item::new("config");
pub const ALLOCATIONS: Map<&Addr, Allocation> = Map::new("allocations");
pub const VOTING_POWER_SNAPSHOTS: Map<&Addr, Vec<Snapshot>> = Map::new("snapshots");

pub const TEMP_DATA: Item<Addr> = Item::new("temp_data");
