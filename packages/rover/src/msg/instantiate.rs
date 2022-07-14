use cw_asset::AssetInfoUnchecked;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InstantiateMsg {
    pub owner: String,
    pub allowed_vaults: Vec<String>,
    pub allowed_assets: Vec<AssetInfoUnchecked>,
}
