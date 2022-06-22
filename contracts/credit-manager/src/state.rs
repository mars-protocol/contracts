use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const OWNER: Item<Addr> = Item::new("owner");

pub const ALLOWED_ASSETS: Map<String, bool> = Map::new("allowed_assets");
pub const ALLOWED_VAULTS: Map<Addr, bool> = Map::new("allowed_vaults");
