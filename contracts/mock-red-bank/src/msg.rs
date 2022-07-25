use cosmwasm_std::Uint128;
use cw_asset::{AssetInfoUnchecked, AssetUnchecked};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Borrow {
        asset: AssetUnchecked,
        recipient: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    UserAssetDebt {
        user_address: String,
        asset: AssetInfoUnchecked,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserAssetDebtResponse {
    pub asset_info: AssetInfoUnchecked,
    pub amount: Uint128,
}
