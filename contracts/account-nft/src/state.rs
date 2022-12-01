use cw_storage_plus::Item;

use crate::config::Config;

pub const CONFIG: Item<Config> = Item::new("config");
pub const NEXT_ID: Item<u64> = Item::new("next_id");
