use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use mars_types::oracle::AstroportTwapSnapshot;

/// The Astroport Factory contract address
pub const ASTROPORT_FACTORY: Item<Addr> = Item::new("astroport_factory");

/// TWAP snapshots indexed by denom
pub const ASTROPORT_TWAP_SNAPSHOTS: Map<&str, Vec<AstroportTwapSnapshot>> = Map::new("snapshots");
