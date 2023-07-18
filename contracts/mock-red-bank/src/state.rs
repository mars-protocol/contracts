use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Map;

// Map<(DebtHolder, CoinDenom), AmountOfDebt>
pub const DEBT_AMOUNT: Map<(Addr, String), Uint128> = Map::new("debt_amount");
// Map<(Addr, CmAccountId, CoinDenom), AmountOfCollateral>
pub const COLLATERAL_AMOUNT: Map<(String, String, String), Uint128> = Map::new("collateral_amount");
// Map<(Addr, CmAccountId), Vec<CoinDenom>> : Used for tracking total denoms user deposited
pub const COLLATERAL_DENOMS: Map<(String, String), Vec<String>> = Map::new("collateral_denoms");
