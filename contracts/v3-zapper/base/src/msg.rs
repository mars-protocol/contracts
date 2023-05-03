use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin};
use mars_owner::OwnerUpdate;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
}

#[cw_serde]
pub struct NewPositionRequest {
    pub pool_id: u64,
    pub lower_tick: i64,
    pub upper_tick: i64,
    pub token_desired0: Option<Coin>,
    pub token_desired1: Option<Coin>,
    pub token_min_amount0: String,
    pub token_min_amount1: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Expects the corresponding tokens to be sent in Funds. The position created will be owned
    /// by the zapper contract itself. Consumer should expect an event with
    /// V3_POSITION_CREATED_EVENT_TYPE & V3_POSITION_ATTR_KEY, used to parse the id of the position
    /// created. In the event not all funds are issued, the remaining is refunded to the caller.
    CreatePosition(NewPositionRequest),
    UpdateOwner(OwnerUpdate),
    Callback(CallbackMsg),
}

#[cw_serde]
pub enum CallbackMsg {
    RefundCoin {
        recipient: Addr,
        denoms: Vec<String>,
    },
}

impl CallbackMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(mars_owner::OwnerResponse)]
    Owner {},
}
