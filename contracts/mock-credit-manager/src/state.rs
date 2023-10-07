use cw_storage_plus::{Item, Map};
use mars_rover::msg::query::{ConfigResponse, Positions};
use mars_rover_health_types::AccountKind;

pub const CONFIG: Item<ConfigResponse> = Item::new("config");

pub const POSITION_RESPONSES: Map<&str, Positions> = Map::new("position_responses"); // Map<account_id, Positions>

pub const ACCOUNT_KINDS: Map<&str, AccountKind> = Map::new("account_types");
