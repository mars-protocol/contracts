use cw_storage_plus::{Item, Map};

use mars_outpost::protocol_rewards_collector::Config;

use crate::msg::SwapInstructions;

// The reward collector contract's config
pub const CONFIG: Item<Config> = Item::new("config");

// Instructions for swapping an offer asset into an ask asset
pub const INSTRUCTIONS: Map<(String, String), SwapInstructions> = Map::new("instructions");
