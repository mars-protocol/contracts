use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_item_set::Set;
use cw_storage_plus::{Item, Map};
use mars_owner::Owner;

use mars_rover::adapters::swap::Swapper;
use mars_rover::adapters::vault::{VaultConfig, VaultPositionAmount};
use mars_rover::adapters::{Oracle, RedBank, Zapper};

use crate::vault::RequestTempStorage;

// Contract dependencies
// NOTE: Ensure assert_not_contract_in_config() is updated when an external contract is added here
pub const ACCOUNT_NFT: Item<Addr> = Item::new("account_nft");
pub const ORACLE: Item<Oracle> = Item::new("oracle");
pub const RED_BANK: Item<RedBank> = Item::new("red_bank");
pub const SWAPPER: Item<Swapper> = Item::new("swapper");
pub const VAULT_CONFIGS: Map<&Addr, VaultConfig> = Map::new("vault_configs");
pub const ZAPPER: Item<Zapper> = Item::new("zapper");

// Config
pub const OWNER: Owner = Owner::new("owner");
pub const ALLOWED_COINS: Set<&str> = Set::new("allowed_coins");
pub const MAX_CLOSE_FACTOR: Item<Decimal> = Item::new("max_close_factor");
pub const MAX_UNLOCKING_POSITIONS: Item<Uint128> = Item::new("max_unlocking_positions");

// Positions
pub const COIN_BALANCES: Map<(&str, &str), Uint128> = Map::new("coin_balance"); // Map<(AccountId, Denom), Amount>
pub const DEBT_SHARES: Map<(&str, &str), Uint128> = Map::new("debt_shares"); // Map<(AccountId, Denom), Shares>
pub const TOTAL_DEBT_SHARES: Map<&str, Uint128> = Map::new("total_debt_shares"); // Map<Denom, Shares>
pub const VAULT_POSITIONS: Map<(&str, Addr), VaultPositionAmount> = Map::new("vault_positions"); // Map<(AccountId, VaultAddr), VaultPositionAmount>

// Temporary state to save variables to be used on reply handling
pub const VAULT_REQUEST_TEMP_STORAGE: Item<RequestTempStorage> =
    Item::new("vault_request_temp_var");
