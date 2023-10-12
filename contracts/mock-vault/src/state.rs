use cosmwasm_std::{Addr, Coin, Uint128};
use cw_storage_plus::{Item, Map};
use cw_utils::Duration;
use cw_vault_standard::extensions::lockup::UnlockingPosition;
use mars_types::adapters::oracle::Oracle;

pub const VAULT_TOKEN_DENOM: Item<String> = Item::new("vault_token_denom");
pub const TOTAL_VAULT_SHARES: Item<Uint128> = Item::new("total_vault_shares");
pub const LOCKUP_TIME: Item<Option<Duration>> = Item::new("lockup_time");
pub const ORACLE: Item<Oracle> = Item::new("oracle");

pub const COIN_BALANCE: Item<Coin> = Item::new("underlying_coin");
pub const UNLOCKING_POSITIONS: Map<Addr, Vec<UnlockingPosition>> = Map::new("unlocking_positions");
pub const NEXT_LOCKUP_ID: Item<u64> = Item::new("next_lockup_id");

// Used to simulate a compromised vault that attempts reentrancy
// String == Credit account evil vault owns
pub const IS_EVIL: Item<Option<String>> = Item::new("is_evil");

// Used for mock LP token minting
pub const CHAIN_BANK: Item<Uint128> = Item::new("chain_bank");
