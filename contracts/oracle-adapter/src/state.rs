use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use mars_rover::adapters::Oracle;

use crate::msg::VaultPricingInfo;

pub const ADMIN: Admin = Admin::new("admin");
pub const ORACLE: Item<Oracle> = Item::new("oracle");

/// Map<(Vault Token Denom, Pricing Method)>
pub const VAULT_PRICING_INFO: Map<&str, VaultPricingInfo> = Map::new("vault_coin");
