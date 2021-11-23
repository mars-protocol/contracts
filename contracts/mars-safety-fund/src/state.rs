use cw_storage_plus::Item;

use crate::Config;

// Key
pub const CONFIG: Item<Config> = Item::new("config");
