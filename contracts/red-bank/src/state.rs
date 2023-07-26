use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use mars_owner::Owner;
use mars_red_bank_types::red_bank::{Collateral, Config, Debt, Market};

pub const OWNER: Owner = Owner::new("owner");
pub const CONFIG: Item<Config<Addr>> = Item::new("config");
pub const MARKETS: Map<&str, Market> = Map::new("markets");
/// The key is: user address, account id (if any), collateral denom
pub const COLLATERALS: Map<(&Addr, &str, &str), Collateral> = Map::new("collaterals");
pub const DEBTS: Map<(&Addr, &str), Debt> = Map::new("debts");
pub const UNCOLLATERALIZED_LOAN_LIMITS: Map<(&Addr, &str), Uint128> = Map::new("limits");
