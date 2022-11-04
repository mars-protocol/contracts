use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use mars_rover::adapters::Oracle;

use crate::msg::VaultPricingInfo;

pub const OWNER: Item<Addr> = Item::new("owner");
pub const ORACLE: Item<Oracle> = Item::new("oracle");

/// Map<(Vault Token Denom, Pricing Method)>
pub const VAULT_PRICING_INFO: Map<&str, VaultPricingInfo> = Map::new("vault_coin");
