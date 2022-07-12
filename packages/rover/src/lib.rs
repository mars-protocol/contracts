use cw_asset::AssetInfoUnchecked;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InstantiateMsg {
    pub owner: String,
    pub allowed_vaults: Vec<String>,
    pub allowed_assets: Vec<AssetInfoUnchecked>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    CreateCreditAccount {},
    UpdateConfig {
        account_nft: Option<String>,
        owner: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Owner & account nft address. Response type: `ConfigResponse`
    Config {},
    /// Whitelisted vaults. Response type: `Vec<String>`
    AllowedVaults {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Whitelisted assets. Response type: `Vec<AssetInfoUnchecked>`
    AllowedAssets {
        start_after: Option<AssetInfoUnchecked>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub owner: String,
    pub account_nft: Option<String>,
}
