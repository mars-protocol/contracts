use cosmwasm_std::{Addr, Order, Storage, Uint128};
use cw_storage_plus::{Item, Map};

use mars_outpost::red_bank::{Collateral, Config, Debt, Market};

pub const CONFIG: Item<Config> = Item::new("config");

pub const MARKETS: Map<&str, Market> = Map::new("markets");
pub const MARKET_DENOMS_BY_MA_TOKEN: Map<&Addr, String> = Map::new("market_denoms_by_ma_token");

pub const COLLATERALS: Map<(&Addr, &str), Collateral> = Map::new("collaterals");
pub const DEBTS: Map<(&Addr, &str), Debt> = Map::new("debts");

pub const UNCOLLATERALIZED_LOAN_LIMITS: Map<(&Addr, &str), Uint128> = Map::new("limits");

/// Return `true` if the user is borrowing a non-zero amount in _any_ asset; return `false` if the
/// user is not borrowing any asset.
///
/// The user is borrowing if, in the `DEBTS` map, there is at least one denom stored under the user
/// address prefix.
pub fn user_is_borrowing(store: &dyn Storage, addr: &Addr) -> bool {
    DEBTS.prefix(addr).range(store, None, None, Order::Ascending).next().is_some()
}
