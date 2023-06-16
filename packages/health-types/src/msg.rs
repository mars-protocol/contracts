use cosmwasm_schema::{cw_serde, QueryResponses};
use mars_owner::{OwnerResponse, OwnerUpdate};

use crate::AccountKind;

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
        credit_manager: Option<String>,
        params: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::HealthResponse)]
    Health {
        account_id: String,
        kind: AccountKind,
    },
    #[returns(ConfigResponse)]
    Config {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub credit_manager: Option<String>,
    pub params: Option<String>,
    pub owner_response: OwnerResponse,
}
