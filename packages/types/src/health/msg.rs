use cosmwasm_schema::{cw_serde, QueryResponses};
use mars_owner::{OwnerResponse, OwnerUpdate};

use super::AccountKind;
use crate::oracle::ActionKind;

#[cw_serde]
pub struct InstantiateMsg {
    /// The address with privileged access to update config
    pub owner: String,

    /// Credit Manager contract address
    pub credit_manager: Option<String>,
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
    #[returns(super::HealthValuesResponse)]
    HealthValues {
        account_id: String,
        kind: AccountKind,
        action: ActionKind,
    },
    /// Returns Healthy or Unhealthy state. Does not do health calculations if no debt.
    /// This is helpful in the cases like liquidation where we should not query the oracle if can help it.
    #[returns(super::HealthState)]
    HealthState {
        account_id: String,
        kind: AccountKind,
        action: ActionKind,
    },
    #[returns(ConfigResponse)]
    Config {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub credit_manager: Option<String>,
    pub owner_response: OwnerResponse,
}
