use cosmwasm_std::Decimal;
use cw_storage_plus::Map;

pub const COIN_PRICE: Map<String, Decimal> = Map::new("coin_price");
