use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::adapters::RedBankUnchecked;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InstantiateMsg {
    pub owner: String,
    pub allowed_vaults: Vec<String>,
    pub allowed_coins: Vec<String>,
    pub red_bank: RedBankUnchecked,
}

/// Used when you want to update fields on Instantiate config
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ConfigUpdates {
    pub account_nft: Option<String>,
    pub owner: Option<String>,
    pub allowed_coins: Option<Vec<String>>,
    pub allowed_vaults: Option<Vec<String>>,
    pub red_bank: Option<RedBankUnchecked>,
}
