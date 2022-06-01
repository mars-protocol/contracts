use cw_storage_plus::Item;

/// The mint contract for the collection. Set on instantiation.
pub const SOME_STRING: Item<String> = Item::new("some_string");
