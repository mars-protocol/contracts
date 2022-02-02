use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

use mars_core::vesting::{Allocation, Config};

pub const CONFIG: Item<Config<Addr>> = Item::new("config");
pub const ALLOCATIONS: Map<&Addr, Allocation> = Map::new("allocations");
