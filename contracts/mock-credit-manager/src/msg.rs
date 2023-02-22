use cosmwasm_schema::cw_serde;
use mars_rover::{
    adapters::vault::VaultConfig,
    msg::query::{ConfigResponse, Positions},
};

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
    SetAllowedCoins(Vec<String>),
    SetVaultConfig {
        address: String,
        config: VaultConfig,
    },
}
