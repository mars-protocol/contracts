use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use mars_owner::Owner;

pub const OWNER: Owner = Owner::new("owner");
pub const CREDIT_MANAGER: Item<Addr> = Item::new("credit_manager");
pub const PARAMS: Item<Addr> = Item::new("params");
