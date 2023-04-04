use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Decimal;
use mars_owner::OwnerUpdate;

use crate::types::{
    AssetParams, AssetParamsUpdate, EmergencyUpdate, VaultConfig, VaultConfigUpdate,
};

#[cw_serde]
pub struct InstantiateMsg {
    /// Contract's owner
    pub owner: String,
    /// The maximum percent a liquidator can decrease the debt amount of the liquidatee
    pub max_close_factor: Decimal,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateOwner(OwnerUpdate),
    UpdateMaxCloseFactor(Decimal),
    UpdateAssetParams(AssetParamsUpdate),
    UpdateVaultConfig(VaultConfigUpdate),
    EmergencyUpdate(EmergencyUpdate),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(mars_owner::OwnerResponse)]
    Owner {},

    #[returns(AssetParams)]
    AssetParams {
        denom: String,
    },

    #[returns(Vec<crate::types::AssetParamsResponse>)]
    AllAssetParams {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(VaultConfig)]
    VaultConfig {
        /// Address of vault
        address: String,
    },

    #[returns(Vec<VaultConfig>)]
    AllVaultConfigs {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(Decimal)]
    MaxCloseFactor {},
}
