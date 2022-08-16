use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct MockEnv {
    pub credit_manager: Addr,
    pub oracle: Addr,
    pub red_bank: Addr,
    pub nft: Addr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct CoinInfo {
    pub denom: String,
    pub price: Decimal,
    pub max_ltv: Decimal,
    pub liquidation_threshold: Decimal,
}

impl CoinInfo {
    pub fn to_coin(&self, amount: Uint128) -> Coin {
        Coin {
            denom: self.denom.clone(),
            amount,
        }
    }
}
