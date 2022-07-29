use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

use mars_outpost::protocol_rewards_collector::Config;

use crate::Route;

// The reward collector contract's config
pub const CONFIG: Item<Config<Addr>> = Item::new("config");

// Instructions for swapping an offer asset into an ask asset
pub const ROUTES: Map<(String, String), Route> = Map::new("routes");
