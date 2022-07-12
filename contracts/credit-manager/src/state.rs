use cosmwasm_std::{Addr, Uint128};
use cw_asset::AssetInfoKey;
use cw_storage_plus::{Item, Map};
use rover::adapters::RedBank;

// Contract config
pub const OWNER: Item<Addr> = Item::new("owner");
pub const ACCOUNT_NFT: Item<Addr> = Item::new("account_nft");
pub const ALLOWED_ASSETS: Map<AssetInfoKey, bool> = Map::new("allowed_assets");
pub const ALLOWED_VAULTS: Map<Addr, bool> = Map::new("allowed_vaults");
pub const RED_BANK: Item<RedBank> = Item::new("red_bank");

// Positions
pub type NftTokenId<'a> = &'a str;
pub const ASSETS: Map<(NftTokenId, AssetInfoKey), Uint128> = Map::new("assets");

type Shares = Uint128;
pub const DEBT_SHARES: Map<(NftTokenId, AssetInfoKey), Shares> = Map::new("debt_shares");
/// Used to calculate each user's share of the debt
pub const TOTAL_DEBT_SHARES: Map<AssetInfoKey, Shares> = Map::new("total_debt_shares");
