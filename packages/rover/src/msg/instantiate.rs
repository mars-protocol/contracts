use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::adapters::{OracleUnchecked, RedBankUnchecked};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InstantiateMsg {
    /// The address with privileged access to update config
    pub owner: String,
    /// Whitelisted coin denoms approved by governance
    pub allowed_coins: Vec<String>,
    /// Whitelisted vaults approved by governance that implement credit manager's vault interface
    pub allowed_vaults: Vec<String>,
    /// The Mars Protocol money market contract where we borrow assets from
    pub red_bank: RedBankUnchecked,
    /// The Mars Protocol oracle contract. We read prices of assets here.
    pub oracle: OracleUnchecked,
}

/// Used when you want to update fields on Instantiate config
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct ConfigUpdates {
    pub account_nft: Option<String>,
    pub owner: Option<String>,
    pub allowed_coins: Option<Vec<String>>,
    pub allowed_vaults: Option<Vec<String>>,
    pub red_bank: Option<RedBankUnchecked>,
    pub oracle: Option<OracleUnchecked>,
}
