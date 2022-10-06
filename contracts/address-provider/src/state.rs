use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

use mars_outpost::address_provider::Config;

use crate::key::MarsAddressKey;

pub const CONFIG: Item<Config> = Item::new("config");
pub const CONTRACTS: Map<MarsAddressKey, Addr> = Map::new("contracts");
pub const GOVERNANCE: Map<MarsAddressKey, String> = Map::new("governance");
