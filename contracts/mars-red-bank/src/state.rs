use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

use mars_outpost::red_bank::{Config, Debt, Market};

/// The contract's configurations
pub const CONFIG: Item<Config> = Item::new("config");

/// Money market for each asset, indexed by denoms
pub const MARKETS: Map<&str, Market> = Map::new("markets");

/// Scaled collateral amounts, indexed by composite key {user_address | denom}
pub const COLLATERALS: Map<(&Addr, &str), Uint128> = Map::new("collaterals");

/// Scaled debt amounts, indexed by composite key {user_address | denom}
pub const DEBTS: Map<(&Addr, &str), Debt> = Map::new("debts");

/// Uncollateralized loan limits, indexed by composite key {denom | user_address}
pub const UNCOLLATERALIZED_LOAN_LIMITS: Map<(&str, &Addr), Uint128> =
    Map::new("uncollateralized_loan_limits");
