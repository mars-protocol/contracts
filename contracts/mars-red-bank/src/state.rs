use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map, U32Key};

use crate::{Config, Debt, GlobalState, Market, User};

pub const CONFIG: Item<Config> = Item::new("config");
pub const GLOBAL_STATE: Item<GlobalState> = Item::new("global_state");

pub const USERS: Map<&Addr, User> = Map::new("users");

pub const MARKETS: Map<&[u8], Market> = Map::new("markets");
pub const MARKET_REFERENCES_BY_INDEX: Map<U32Key, Vec<u8>> = Map::new("market_refs_by_index");
pub const MARKET_REFERENCES_BY_MA_TOKEN: Map<&Addr, Vec<u8>> = Map::new("market_refs_by_ma_token");

pub const DEBTS: Map<(&[u8], &Addr), Debt> = Map::new("debts");
pub const UNCOLLATERALIZED_LOAN_LIMITS: Map<(&[u8], &Addr), Uint128> =
    Map::new("uncollateralized_loan_limits");
