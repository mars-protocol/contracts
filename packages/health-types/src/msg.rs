use cosmwasm_schema::{cw_serde, QueryResponses};
use mars_owner::{OwnerResponse, OwnerUpdate};

#[cw_serde]
pub struct InstantiateMsg {
    /// The address with privileged access to update config
    pub owner: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Manages owner role state
    UpdateOwner(OwnerUpdate),
    /// Update contract config constants
    UpdateConfig {
        credit_manager: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::HealthResponse)]
    Health {
        account_id: String,
    },
    #[returns(ConfigResponse)]
    Config {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub credit_manager_addr: Option<String>,
    pub owner_response: OwnerResponse,
}
