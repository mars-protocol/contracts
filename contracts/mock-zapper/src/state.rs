use cosmwasm_std::{Coin, Uint128};
use cw_storage_plus::{Item, Map};
use rover::adapters::Oracle;

pub const ORACLE: Item<Oracle> = Item::new("oracle");

pub const LP_TOKEN_SUPPLY: Map<&str, Uint128> = Map::new("lp_token_supply"); // LP token denom -> Total LP token supply
pub const COIN_BALANCES: Map<&str, (Coin, Coin)> = Map::new("coin_balances"); // LP token denom -> Underlying tokens
