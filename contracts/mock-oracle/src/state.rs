use cosmwasm_std::Decimal;
use cw_storage_plus::Map;

pub const DEFAULT_COIN_PRICE: Map<String, Decimal> = Map::new("default_coin_price");
pub const LIQUIDATION_COIN_PRICE: Map<String, Decimal> = Map::new("liquidation_coin_price");
