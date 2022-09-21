use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const PENDING_OWNER: Item<Addr> = Item::new("pending_owner");
pub const NEXT_ID: Item<u64> = Item::new("next_id");
