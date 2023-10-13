use cosmwasm_std::{Addr, Decimal};
use cw_storage_plus::{Item, Map};
use mars_owner::Owner;
use mars_types::params::{AssetParams, VaultConfig};

pub const OWNER: Owner = Owner::new("owner");
pub const ADDRESS_PROVIDER: Item<Addr> = Item::new("address_provider");
pub const ASSET_PARAMS: Map<&str, AssetParams> = Map::new("asset_params");
pub const VAULT_CONFIGS: Map<&Addr, VaultConfig> = Map::new("vault_configs");
pub const TARGET_HEALTH_FACTOR: Item<Decimal> = Item::new("target_health_factor");
