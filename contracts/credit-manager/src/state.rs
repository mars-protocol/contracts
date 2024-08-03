use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};
use mars_owner::Owner;
use mars_types::{
    adapters::{
        account_nft::AccountNft, health::HealthContract, incentives::Incentives, oracle::Oracle,
        params::Params, red_bank::RedBank, rewards_collector::RewardsCollector, swapper::Swapper,
        vault::VaultPositionAmount, zapper::Zapper,
    },
    health::AccountKind,
};
use mars_utils::guard::Guard;

use crate::vault::RequestTempStorage;

// Contract dependencies
pub const ACCOUNT_NFT: Item<AccountNft> = Item::new("account_nft");
pub const ORACLE: Item<Oracle> = Item::new("oracle");
pub const RED_BANK: Item<RedBank> = Item::new("red_bank");
pub const SWAPPER: Item<Swapper> = Item::new("swapper");
pub const ZAPPER: Item<Zapper> = Item::new("zapper");
pub const HEALTH_CONTRACT: Item<HealthContract> = Item::new("health_contract");
pub const PARAMS: Item<Params> = Item::new("params");
pub const INCENTIVES: Item<Incentives> = Item::new("incentives");

// Config
pub const OWNER: Owner = Owner::new("owner");
pub const MAX_UNLOCKING_POSITIONS: Item<Uint128> = Item::new("max_unlocking_positions");
pub const REENTRANCY_GUARD: Guard = Guard::new("reentrancy_guard");
pub const MAX_SLIPPAGE: Item<Decimal> = Item::new("max_slippage");

// Positions
pub const ACCOUNT_KINDS: Map<&str, AccountKind> = Map::new("account_types"); // Map<AccountId, AccountKind>
pub const COIN_BALANCES: Map<(&str, &str), Uint128> = Map::new("coin_balance"); // Map<(AccountId, Denom), Amount>
pub const DEBT_SHARES: Map<(&str, &str), Uint128> = Map::new("debt_shares"); // Map<(AccountId, Denom), Shares>
pub const TOTAL_DEBT_SHARES: Map<&str, Uint128> = Map::new("total_debt_shares"); // Map<Denom, Shares>

pub const VAULT_POSITIONS: Map<(&str, Addr), VaultPositionAmount> = Map::new("vault_positions"); // Map<(AccountId, VaultAddr), VaultPositionAmount>

// Temporary state to save variables to be used on reply handling
pub const VAULT_REQUEST_TEMP_STORAGE: Item<RequestTempStorage> =
    Item::new("vault_request_temp_var");

// (account id, addr) for rewards-collector contract
pub const REWARDS_COLLECTOR: Item<RewardsCollector> = Item::new("rewards_collector");

// (account id, vault addr) bindings between account and vault
pub const VAULTS: Map<&str, Addr> = Map::new("vaults");
