use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Map;

// Map<(DebtHolder, CoinDenom), AmountOfDebt>
pub const DEBT_AMOUNT: Map<(Addr, String), Uint128> = Map::new("debt_amount");
// Map<(CollateralHolder, CoinDenom), AmountOfCollateral>
pub const COLLATERAL_AMOUNT: Map<(Addr, String), Uint128> = Map::new("collateral_amount");
