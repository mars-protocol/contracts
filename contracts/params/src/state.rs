use cosmwasm_std::{Addr, Decimal};
use cw_storage_plus::{Item, Map};
use mars_owner::Owner;

use crate::types::{asset::AssetParams, vault::VaultConfig};

pub const OWNER: Owner = Owner::new("owner");
pub const ASSET_PARAMS: Map<&str, AssetParams> = Map::new("asset_params");
pub const VAULT_CONFIGS: Map<&Addr, VaultConfig> = Map::new("vault_configs");
pub const MAX_CLOSE_FACTOR: Item<Decimal> = Item::new("max_close_factor");
