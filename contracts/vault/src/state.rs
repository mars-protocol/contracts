use cw_storage_plus::Item;
use mars_owner::Owner;

pub const OWNER: Owner = Owner::new("owner");

pub const CREDIT_MANAGER: Item<String> = Item::new("cm_addr");
pub const FOUND_MANAGER_ACC_ID: Item<String> = Item::new("fm_acc_id");

pub const TITLE: Item<String> = Item::new("fm_title");
pub const SUBTITLE: Item<String> = Item::new("fm_subtitle");
pub const DESCRIPTION: Item<String> = Item::new("fm_desc");
