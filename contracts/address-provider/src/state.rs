use cw_storage_plus::{Item, Map};

use mars_outpost::address_provider::Config;

use crate::key::MarsContractKey;

pub const CONFIG: Item<Config> = Item::new("config");
pub const CONTRACTS: Map<MarsContractKey, String> = Map::new("contracts");
