use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal;

#[cw_serde]
pub struct InstantiateMsg {
    pub coins: Vec<CoinMarketInfo>,
}

#[cw_serde]
pub struct CoinMarketInfo {
    pub denom: String,
    pub max_ltv: Decimal,
    pub liquidation_threshold: Decimal,
}
