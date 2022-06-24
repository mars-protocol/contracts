use cw_asset::AssetInfoUnchecked;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InstantiateMsg {
    pub owner: String,
    pub allowed_vaults: Vec<String>,
    pub allowed_assets: Vec<AssetInfoUnchecked>,
    pub nft_contract_code_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    CreateCreditAccount {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// The contract's owner. Response type: `String`
    Owner {},
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
    CreditAccountNftAddress {},
}
