use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

use mars_outpost::red_bank::{Collateral, Config, Debt, Market};

pub const CONFIG: Item<Config> = Item::new("config");

pub const MARKETS: Map<&str, Market> = Map::new("markets");
pub const MARKET_DENOMS_BY_MA_TOKEN: Map<&Addr, String> = Map::new("market_denoms_by_ma_token");

pub const COLLATERALS: Map<(&Addr, &str), Collateral> = Map::new("collaterals");
pub const DEBTS: Map<(&Addr, &str), Debt> = Map::new("debts");

pub const UNCOLLATERALIZED_LOAN_LIMITS: Map<(&Addr, &str), Uint128> = Map::new("limits");
