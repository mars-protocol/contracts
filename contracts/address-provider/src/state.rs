use cw_controllers_admin_fork::Admin;
use cw_storage_plus::{Item, Map};

use mars_outpost::address_provider::Config;

use crate::key::MarsAddressTypeKey;

pub const OWNER: Admin = Admin::new("owner");
pub const CONFIG: Item<Config> = Item::new("config");
pub const ADDRESSES: Map<MarsAddressTypeKey, String> = Map::new("addresses");
