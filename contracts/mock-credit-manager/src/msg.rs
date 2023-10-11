use cosmwasm_schema::cw_serde;
use mars_rover::msg::query::{ConfigResponse, Positions};
use mars_rover_health_types::AccountKind;

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
    SetAccountKindResponse {
        account_id: String,
        kind: AccountKind,
    },
}
