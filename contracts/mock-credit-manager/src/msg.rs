use cosmwasm_schema::cw_serde;
use mars_rover::{
    adapters::vault::VaultConfig,
    msg::query::{ConfigResponse, Positions},
};
use mars_rover_health_types::HealthResponse;

#[cw_serde]
pub struct InstantiateMsg {
    pub config: ConfigResponse,
}

#[cw_serde]
pub enum ExecuteMsg {
    SetHealthResponse {
        account_id: String,
        response: HealthResponse,
    },
    SetPositionsResponse {
        account_id: String,
        positions: Positions,
    },
    SetAllowedCoins(Vec<String>),
    SetVaultConfig {
        address: String,
        config: VaultConfig,
    },
}
