use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Decimal, Uint128};

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

#[cw_serde]
pub enum ExecuteMsg {
    Borrow {
        coin: Coin,
        recipient: Option<String>,
    },
    Repay {
        denom: String,
        on_behalf_of: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(UserAssetDebtResponse)]
    UserAssetDebt { user_address: String, denom: String },
    #[returns(mars_outpost::red_bank::Market)]
    Market { denom: String },
}

#[cw_serde]
pub struct UserAssetDebtResponse {
    pub denom: String,
    pub amount: Uint128,
}
