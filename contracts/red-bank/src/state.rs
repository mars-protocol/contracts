use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

use mars_outpost::red_bank::{Config, Debt, GlobalState, Market, User};

pub const CONFIG: Item<Config> = Item::new("config");
pub const GLOBAL_STATE: Item<GlobalState> = Item::new("global_state");

pub const USERS: Map<&Addr, User> = Map::new("users");

pub const MARKETS: Map<&str, Market> = Map::new("markets");
pub const MARKET_DENOMS_BY_INDEX: Map<u32, String> = Map::new("market_denoms_by_index");
pub const MARKET_DENOMS_BY_MA_TOKEN: Map<&Addr, String> = Map::new("market_denoms_by_ma_token");

pub const DEBTS: Map<(&str, &Addr), Debt> = Map::new("debts");
pub const UNCOLLATERALIZED_LOAN_LIMITS: Map<(&str, &Addr), Uint128> =
    Map::new("uncollateralized_loan_limits");
