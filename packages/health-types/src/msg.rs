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
        credit_manager: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns all values that comprise health for account
    #[returns(crate::HealthValuesResponse)]
    HealthValues {
        account_id: String,
        kind: AccountKind,
    },
    /// Returns Healthy or Unhealthy state. Does not do health calculations if no debt.
    /// This is helpful in the cases like liquidation where we should not query the oracle if can help it.
    #[returns(crate::HealthState)]
    HealthState {
        account_id: String,
        kind: AccountKind,
    },
    #[returns(ConfigResponse)]
    Config {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub credit_manager: Option<String>,
    pub owner_response: OwnerResponse,
}
