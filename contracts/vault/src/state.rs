use cw_storage_plus::Item;
use mars_owner::Owner;

pub const OWNER: Owner = Owner::new("owner");

pub const CREDIT_MANAGER: Item<String> = Item::new("cm_addr");
pub const VAULT_ACC_ID: Item<String> = Item::new("cm_acc_id");

pub const TITLE: Item<String> = Item::new("title");
pub const SUBTITLE: Item<String> = Item::new("subtitle");
pub const DESCRIPTION: Item<String> = Item::new("desc");
