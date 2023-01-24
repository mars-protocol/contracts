use cw_storage_plus::{Item, Map};
use mars_owner::Owner;
use mars_types::address_provider::Config;

use crate::key::MarsAddressTypeKey;

pub const OWNER: Owner = Owner::new("owner");
pub const CONFIG: Item<Config> = Item::new("config");
pub const ADDRESSES: Map<MarsAddressTypeKey, String> = Map::new("addresses");
