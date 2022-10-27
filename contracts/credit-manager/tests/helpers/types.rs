use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, Addr, Coin, Decimal};

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
pub struct VaultTestInfo {
    pub denom: String,
    pub lockup: Option<u64>,
    pub underlying_denoms: Vec<String>,
    pub deposit_cap: Coin,
    pub max_ltv: Decimal,
    pub liquidation_threshold: Decimal,
}

impl CoinInfo {
    pub fn to_coin(&self, amount: u128) -> Coin {
        coin(amount, self.denom.clone())
    }
}
