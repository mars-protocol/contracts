use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::Map;

// Map<(DebtHolder, CoinDenom), AmountOfDebt>
pub const DEBT_AMOUNT: Map<(Addr, String), Uint128> = Map::new("debt_amount");
// Map<CoinDenom, Shares>
pub const ASSET_LTV: Map<String, Decimal> = Map::new("asset_ltv");
