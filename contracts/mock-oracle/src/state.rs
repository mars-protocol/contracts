use cosmwasm_std::Decimal;
use cw_storage_plus::Map;

pub const ASSET_PRICE: Map<String, Decimal> = Map::new("asset_price");
