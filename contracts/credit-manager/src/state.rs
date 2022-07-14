use cosmwasm_std::{Addr, Uint128};
use cw_asset::AssetInfoKey;
use cw_storage_plus::{Item, Map};

// Contract config
pub const OWNER: Item<Addr> = Item::new("owner");
pub const ACCOUNT_NFT: Item<Addr> = Item::new("account_nft");
pub const ALLOWED_ASSETS: Map<AssetInfoKey, bool> = Map::new("allowed_assets");
pub const ALLOWED_VAULTS: Map<Addr, bool> = Map::new("allowed_vaults");

// Positions
type NftTokenId<'a> = &'a str;
pub const ASSETS: Map<(NftTokenId, AssetInfoKey), Uint128> = Map::new("assets");
