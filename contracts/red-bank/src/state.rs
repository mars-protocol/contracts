use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use mars_owner::Owner;
use mars_types::{
    keys::UserIdKey,
    red_bank::{Collateral, Config, Debt, Market},
};
use mars_utils::guard::Guard;

pub const OWNER: Owner = Owner::new("owner");
pub const CONFIG: Item<Config<Addr>> = Item::new("config");
pub const MARKETS: Map<&str, Market> = Map::new("markets");
pub const COLLATERALS: Map<(&UserIdKey, &str), Collateral> = Map::new("colls");
pub const DEBTS: Map<(&Addr, &str), Debt> = Map::new("debts");

/// Used to mark the contract as locked during migrations
pub const MIGRATION_GUARD: Guard = Guard::new("guard");
