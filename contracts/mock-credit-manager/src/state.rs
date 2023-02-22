use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use mars_rover::{
    adapters::vault::VaultConfig,
    msg::query::{ConfigResponse, Positions},
};

pub const CONFIG: Item<ConfigResponse> = Item::new("config");
pub const ALLOWED_COINS: Item<Vec<String>> = Item::new("allowed_coins"); // Vec<Coin Denom>
pub const VAULT_CONFIGS: Map<&Addr, VaultConfig> = Map::new("vault_configs");

pub const POSITION_RESPONSES: Map<&str, Positions> = Map::new("position_responses"); // Map<account_id, Positions>
