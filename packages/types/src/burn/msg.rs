use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    BurnFunds {
        denom: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(BurntAmountResponse)]
    GetBurntAmount {
        denom: String,
    },
    #[returns(BurntAmountsResponse)]
    GetAllBurntAmounts {
        start_after: Option<String>,
        limit: Option<u8>,
    },
}

#[cw_serde]
pub struct BurntAmountResponse {
    pub denom: String,
    pub amount: Uint128,
}

#[cw_serde]
pub struct BurntAmountsResponse {
    pub burnt_amounts: Vec<BurntAmountResponse>,
}
