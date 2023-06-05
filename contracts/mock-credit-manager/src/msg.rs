use cosmwasm_schema::cw_serde;
use mars_rover::msg::query::{ConfigResponse, Positions};

#[cw_serde]
pub struct InstantiateMsg {
    pub config: ConfigResponse,
}

#[cw_serde]
pub enum ExecuteMsg {
    SetPositionsResponse {
        account_id: String,
        positions: Positions,
    },
}
