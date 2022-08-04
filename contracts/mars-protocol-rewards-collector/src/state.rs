use cw_storage_plus::{Item, Map};

use crate::{AssetConfig, Config};

pub const CONFIG: Item<Config> = Item::new("config");
pub const ASSET_CONFIG: Map<&[u8], AssetConfig> = Map::new("assets");
