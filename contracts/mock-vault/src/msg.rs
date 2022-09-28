use cosmwasm_schema::cw_serde;

use rover::adapters::OracleUnchecked;

// Remaining messages in packages/rover/msg/vault
#[cw_serde]
pub struct InstantiateMsg {
    /// Denom for vault LP share token
    pub lp_token_denom: String,
    /// Denoms for assets in vault
    pub asset_denoms: Vec<String>,
    /// Time in seconds for unlock period
    pub lockup: Option<u64>,
    pub oracle: OracleUnchecked,
}
