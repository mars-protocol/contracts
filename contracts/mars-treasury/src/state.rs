use cw_storage_plus::Item;

use crate::Config;

/// Stores config at the given key
pub const CONFIG: Item<Config> = Item::new("config");
