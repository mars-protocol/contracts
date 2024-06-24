use cw_storage_plus::{Item, Map};
use mars_owner::Owner;

use crate::{
    msg::UnlockState,
    performance_fee::{PerformanceFeeConfig, PerformanceFeeState},
    token_factory::TokenFactoryDenom,
};

pub const OWNER: Owner = Owner::new("owner");

/// The vault token implementation for this vault
pub const VAULT_TOKEN: Item<TokenFactoryDenom> = Item::new("vault_token");

/// The token that is depositable to the vault
pub const BASE_TOKEN: Item<String> = Item::new("base_token");

pub const CREDIT_MANAGER: Item<String> = Item::new("cm_addr");
pub const VAULT_ACC_ID: Item<String> = Item::new("vault_acc_id");

pub const TITLE: Item<String> = Item::new("title");
pub const SUBTITLE: Item<String> = Item::new("subtitle");
pub const DESCRIPTION: Item<String> = Item::new("desc");

pub const COOLDOWN_PERIOD: Item<u64> = Item::new("cooldown_period");
pub const UNLOCKS: Map<(&str, u64), UnlockState> = Map::new("unlocks");

pub const PERFORMANCE_FEE_CONFIG: Item<PerformanceFeeConfig> = Item::new("performance_fee_config");
pub const PERFORMANCE_FEE_STATE: Item<PerformanceFeeState> = Item::new("performance_fee_state");
