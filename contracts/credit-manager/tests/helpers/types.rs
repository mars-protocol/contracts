use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AccountToFund {
    pub addr: Addr,
    pub funds: Vec<Coin>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct CoinInfo {
    pub denom: String,
    pub price: Decimal,
    pub max_ltv: Decimal,
    pub liquidation_threshold: Decimal,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct VaultTestInfo {
    pub lp_token_denom: String,
    pub lockup: Option<u64>,
    pub asset_denoms: Vec<String>,
}

impl CoinInfo {
    pub fn to_coin(&self, amount: Uint128) -> Coin {
        Coin {
            denom: self.denom.clone(),
            amount,
        }
    }
}
