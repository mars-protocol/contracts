use cw_storage_plus::{Item, Map};

use crate::{Config, PriceSourceChecked};

pub const CONFIG: Item<Config> = Item::new("config");
pub const PRICE_SOURCES: Map<&[u8], PriceSourceChecked> = Map::new("price_configs");
