use cosmwasm_schema::cw_serde;
use mars_rover_health_types::{AccountKind, HealthValuesResponse};

#[cw_serde]
pub enum ExecuteMsg {
    SetHealthResponse {
        account_id: String,
        kind: AccountKind,
        response: HealthValuesResponse,
    },
}
