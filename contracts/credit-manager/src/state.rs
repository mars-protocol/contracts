use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const OWNER: Item<Addr> = Item::new("owner");

// e.g. cw20:osmo23905809 or native:uosmo
type AssetInfoStr = String;
pub const ALLOWED_ASSETS: Map<AssetInfoStr, bool> = Map::new("allowed_assets");
pub const ALLOWED_VAULTS: Map<Addr, bool> = Map::new("allowed_vaults");
