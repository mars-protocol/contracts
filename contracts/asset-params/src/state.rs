use cosmwasm_std::{Addr, Decimal};
use cw_storage_plus::{Item, Map};
use mars_owner::Owner;
use crate::types::{AssetParams, VaultConfigs};

pub const OWNER: Owner = Owner::new("owner");
pub const CLOSE_FACTOR: Item<Decimal> = Item::new("max_close_factor");
pub const ASSET_PARAMS: Map<&str, AssetParams> = Map::new("asset_params");
pub const VAULT_CONFIGS: Map<&Addr, VaultConfigs> = Map::new("vault_configs");
