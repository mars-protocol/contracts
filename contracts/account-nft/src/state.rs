use cosmwasm_schema::cw_serde;
use cw_storage_plus::Item;
use mars_account_nft_types::nft_config::NftConfig;

pub const CONFIG: Item<NftConfig> = Item::new("config");
pub const NEXT_ID: Item<u64> = Item::new("next_id");

/// Helper marker used during burning empty accounts. Used only for v1 -> v2 migration.
#[cw_serde]
pub enum BurningMarker {
    StartAfter(String),
    Finished,
}
pub const MIGRATION_BURNING_MARKER: Item<BurningMarker> = Item::new("burning_marker");
