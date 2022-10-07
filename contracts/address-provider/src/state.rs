use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

use mars_outpost::address_provider::Config;

use crate::key::MarsAddressKey;

pub const CONFIG: Item<Config> = Item::new("config");
pub const LOCAL_ADDRESSES: Map<MarsAddressKey, Addr> = Map::new("local_addresses");
pub const REMOTE_ADDRESSES: Map<MarsAddressKey, String> = Map::new("remote_addresses");
