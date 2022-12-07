use cosmwasm_std::Uint128;
use cw_storage_plus::{Item, Map};

use mars_rover::adapters::Oracle;

pub const ORACLE: Item<Oracle> = Item::new("oracle");

pub const LP_TOKEN_SUPPLY: Map<&str, Uint128> = Map::new("lp_token_supply"); // lp token denom -> total lp token supply
pub const COIN_CONFIG: Map<&str, Vec<String>> = Map::new("coin_config"); // lp token denom -> Vec<underlying>
pub const COIN_BALANCES: Map<(&str, &str), Uint128> = Map::new("coin_balances"); // (lp token denom, underlying) -> amount
