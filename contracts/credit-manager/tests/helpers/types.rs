use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, Addr, Coin, Decimal};
use cw_utils::Duration;

#[cw_serde]
pub struct AccountToFund {
    pub addr: Addr,
    pub funds: Vec<Coin>,
}

#[cw_serde]
pub struct CoinInfo {
    pub denom: String,
    pub price: Decimal,
    pub max_ltv: Decimal,
    pub liquidation_threshold: Decimal,
}

#[cw_serde]
pub struct LpCoinInfo {
    pub denom: String,
    pub price: Decimal,
    pub max_ltv: Decimal,
    pub liquidation_threshold: Decimal,
    pub underlying_pair: (String, String),
}

#[cw_serde]
pub struct VaultTestInfo {
    pub vault_token_denom: String,
    pub base_token_denom: String,
    pub lockup: Option<Duration>,
    pub deposit_cap: Coin,
    pub max_ltv: Decimal,
    pub liquidation_threshold: Decimal,
}

impl CoinInfo {
    pub fn to_coin(&self, amount: u128) -> Coin {
        coin(amount, self.denom.clone())
    }
}
