use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

use mars_outpost::oracle::Config;

use crate::msg::PriceSource;

pub const CONFIG: Item<Config<Addr>> = Item::new("config");
pub const PRICE_SOURCES: Map<String, PriceSource> = Map::new("price_sources");
