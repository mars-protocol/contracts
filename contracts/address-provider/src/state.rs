use cw_storage_plus::{Item, Map};
use mars_outpost::address_provider::Config;

use crate::key::MarsAddressTypeKey;

pub const CONFIG: Item<Config> = Item::new("config");
pub const ADDRESSES: Map<MarsAddressTypeKey, String> = Map::new("addresses");
