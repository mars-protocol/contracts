use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct RewardsCollector {
    pub address: String,
    pub account_id: String,
}
