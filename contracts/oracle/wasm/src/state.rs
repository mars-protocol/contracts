use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const ASTROPORT_FACTORY: Item<Addr> = Item::new("astroport_factory");
