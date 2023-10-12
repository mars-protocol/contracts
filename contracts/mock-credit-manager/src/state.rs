use cw_storage_plus::{Item, Map};
use mars_types::{
    credit_manager::{ConfigResponse, Positions},
    health::AccountKind,
};

pub const CONFIG: Item<ConfigResponse> = Item::new("config");

pub const POSITION_RESPONSES: Map<&str, Positions> = Map::new("position_responses"); // Map<account_id, Positions>

pub const ACCOUNT_KINDS: Map<&str, AccountKind> = Map::new("account_types");
