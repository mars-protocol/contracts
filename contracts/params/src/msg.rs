use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Decimal;
use mars_owner::OwnerUpdate;

use crate::types::{asset::AssetParamsUnchecked, vault::VaultConfigUnchecked};

#[cw_serde]
pub struct InstantiateMsg {
    /// Contract's owner
    pub owner: String,
    /// Determines the ideal HF a position should be left at immediately after the position has been liquidated.
    pub target_health_factor: Decimal,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateOwner(OwnerUpdate),
    UpdateTargetHealthFactor(Decimal),
    UpdateAssetParams(AssetParamsUpdate),
    UpdateVaultConfig(VaultConfigUpdate),
    EmergencyUpdate(EmergencyUpdate),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(mars_owner::OwnerResponse)]
    Owner {},

    #[returns(crate::types::asset::AssetParams)]
    AssetParams {
        denom: String,
    },

    #[returns(Vec<crate::types::asset::AssetParams>)]
    AllAssetParams {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(crate::types::vault::VaultConfig)]
    VaultConfig {
        /// Address of vault
        address: String,
    },

    #[returns(Vec<crate::types::vault::VaultConfig>)]
    AllVaultConfigs {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(Decimal)]
    TargetHealthFactor {},
}

#[cw_serde]
pub enum AssetParamsUpdate {
    AddOrUpdate {
        params: AssetParamsUnchecked,
    },
}

#[cw_serde]
pub enum VaultConfigUpdate {
    AddOrUpdate {
        config: VaultConfigUnchecked,
    },
}

#[cw_serde]
pub enum CmEmergencyUpdate {
    SetZeroMaxLtvOnVault(String),
    SetZeroDepositCapOnVault(String),
    DisallowCoin(String),
}

#[cw_serde]
pub enum RedBankEmergencyUpdate {
    DisableBorrowing(String),
}

#[cw_serde]
pub enum EmergencyUpdate {
    CreditManager(CmEmergencyUpdate),
    RedBank(RedBankEmergencyUpdate),
}
