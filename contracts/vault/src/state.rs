use cw_storage_plus::{Item, Map};
use mars_owner::Owner;
use mars_types::adapters::{health::HealthContract, oracle::Oracle};

use crate::msg::UnlockState;

pub const OWNER: Owner = Owner::new("owner");

pub const CREDIT_MANAGER: Item<String> = Item::new("cm_addr");
pub const VAULT_ACC_ID: Item<String> = Item::new("vault_acc_id");

pub const ORACLE: Item<Oracle> = Item::new("oracle");
pub const HEALTH: Item<HealthContract> = Item::new("health");

pub const TITLE: Item<String> = Item::new("title");
pub const SUBTITLE: Item<String> = Item::new("subtitle");
pub const DESCRIPTION: Item<String> = Item::new("desc");

pub const COOLDOWN_PERIOD: Item<u64> = Item::new("cooldown_period");
pub const UNLOCKS: Map<String, Vec<UnlockState>> = Map::new("unlocks");
