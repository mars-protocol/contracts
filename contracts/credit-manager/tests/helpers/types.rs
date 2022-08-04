use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MockEnv {
    pub credit_manager: Addr,
    pub oracle: Addr,
    pub red_bank: Addr,
    pub nft: Addr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct CoinPriceLTV {
    pub denom: String,
    pub price: Decimal,
    pub max_ltv: Decimal,
}

impl CoinPriceLTV {
    pub fn to_coin(&self, amount: Uint128) -> Coin {
        Coin {
            denom: self.denom.clone(),
            amount,
        }
    }
}
