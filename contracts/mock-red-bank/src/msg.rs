use cosmwasm_std::Uint128;
use cw_asset::{Asset, AssetInfoUnchecked};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Borrow {
        asset: Asset,
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
    pub denom: String,
    pub asset_label: String,
    pub asset_reference: Vec<u8>,
    pub asset_info: AssetInfoUnchecked,
    pub amount_scaled: Uint128,
    pub amount: Uint128,
}
