use cw_storage_plus::Item;

use mars_outpost::liquidation_filter::Config;

// keys (for singleton)
pub const CONFIG: Item<Config> = Item::new("config");
