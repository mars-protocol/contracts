use cosmwasm_schema::cw_serde;
use cw_utils::Duration;
use mars_rover::adapters::oracle::OracleUnchecked;

// Remaining messages in cw-vault-standard
#[cw_serde]
pub struct InstantiateMsg {
    /// Denom for vault token
    pub vault_token_denom: String,
    /// Denom required for entry. Also denom received on withdraw.
    pub base_token_denom: String,
    /// Duration of unlock period
    pub lockup: Option<Duration>,
    pub oracle: OracleUnchecked,
}
