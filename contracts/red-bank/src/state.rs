use cosmwasm_std::{Addr, Order, StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map};

use mars_outpost::red_bank::{Collateral, Config, Market};

pub const CONFIG: Item<Config> = Item::new("config");

pub const MARKETS: Map<&str, Market> = Map::new("markets");
pub const MARKET_DENOMS_BY_MA_TOKEN: Map<&Addr, String> = Map::new("market_denoms_by_ma_token");

pub const COLLATERALS: Map<(&Addr, &str), Collateral> = Map::new("collaterals");
pub const DEBTS: Map<(&Addr, &str), Uint128> = Map::new("debts");

pub const UNCOLLATERALIZED_LOAN_LIMITS: Map<(&Addr, &str), Uint128> = Map::new("limits");

/// Return `true` if the user is borrowing a non-zero amount in _any_ asset; return `false` if the
/// user is not borrowing any asset.
///
/// The user is borrowing if, in the `DEBTS` map, there is at least one denom stored under the user
/// address prefix.
pub fn user_is_borrowing(store: &dyn Storage, addr: &Addr) -> bool {
    DEBTS.prefix(addr).range(store, None, None, Order::Ascending).next().is_some()
}

/// Load a user's uncollateralized loan limit. If the user has not been granted a limit, return zero
/// instead of throwing an "not found" error.
pub fn uncollateral_loan_limit(
    store: &dyn Storage,
    addr: &Addr,
    denom: &str,
) -> StdResult<Uint128> {
    Ok(UNCOLLATERALIZED_LOAN_LIMITS.may_load(store, (addr, denom))?.unwrap_or_else(Uint128::zero))
}
