/// state: contains state specific to ma_token (not included in cw20_base)
use cw_storage_plus::Item;

use crate::Config;

pub const CONFIG: Item<Config> = Item::new("config");
