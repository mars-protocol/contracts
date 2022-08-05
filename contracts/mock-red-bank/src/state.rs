use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Map;

use crate::msg::CoinMarketInfo;

// Map<(DebtHolder, CoinDenom), AmountOfDebt>
pub const DEBT_AMOUNT: Map<(Addr, String), Uint128> = Map::new("debt_amount");
// Map<CoinDenom, CoinMarketInfo>
pub const COIN_MARKET_INFO: Map<String, CoinMarketInfo> = Map::new("coin_market_info");
