use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map, U64Key};

use crate::{Claim, Config, GlobalState, SlashEvent};

pub const CONFIG: Item<Config> = Item::new("config");
pub const GLOBAL_STATE: Item<GlobalState> = Item::new("global_state");

pub const CLAIMS: Map<&Addr, Claim> = Map::new("claims");
pub const SLASH_EVENTS: Map<U64Key, SlashEvent> = Map::new("slash_events");
