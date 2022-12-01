use cosmwasm_schema::cw_serde;
use mars_rover::msg::query::HealthResponse;

#[cw_serde]
pub enum ExecuteMsg {
    SetHealthResponse {
        account_id: String,
        response: HealthResponse,
    },
}
