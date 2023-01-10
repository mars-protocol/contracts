use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use mars_outpost::red_bank::{Collateral, Config, Debt, Market};
use mars_owner::Owner;

pub const OWNER: Owner = Owner::new("owner");
pub const EMERGENCY_OWNER: Owner = Owner::new("emergency_owner");
pub const CONFIG: Item<Config<Addr>> = Item::new("config");
pub const MARKETS: Map<&str, Market> = Map::new("markets");
pub const COLLATERALS: Map<(&Addr, &str), Collateral> = Map::new("collaterals");
pub const DEBTS: Map<(&Addr, &str), Debt> = Map::new("debts");
pub const UNCOLLATERALIZED_LOAN_LIMITS: Map<(&Addr, &str), Uint128> = Map::new("limits");
