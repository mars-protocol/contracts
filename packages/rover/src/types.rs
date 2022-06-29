use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// TODO: Local AssetInfo should be replaced by cw-asset when fix is merged on that side
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetInfo {
    Cw20(Addr),
    Native(String),
}

impl ToString for AssetInfo {
    fn to_string(&self) -> String {
        match self {
            AssetInfo::Cw20(addr) => format!("cw20:{}", addr.as_str()),
            AssetInfo::Native(denom) => format!("native:{}", denom.as_str()),
        }
    }
}

impl AssetInfo {
    pub fn from_str(asset_str: String) -> Self {
        let words: Vec<&str> = asset_str.split(':').collect();

        match words[0] {
            "native" => Self::Native(String::from(words[1])),
            "cw20" => Self::Cw20(Addr::unchecked(words[1])),
            asset_type => panic!("{} is not a valid asset type", asset_type),
        }
    }
}
