use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};
use mars_owner::OwnerUpdate;

use super::{asset::AssetParamsUnchecked, vault::VaultConfigUnchecked};

#[cw_serde]
pub struct InstantiateMsg {
    /// Contract's owner
    pub owner: String,
    /// Address of the address provider contract
    pub address_provider: String,
    /// Determines the ideal HF a position should be left at immediately after the position has been liquidated.
    pub target_health_factor: Decimal,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateOwner(OwnerUpdate),
    UpdateConfig {
        address_provider: Option<String>,
    },
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

    #[returns(super::msg::ConfigResponse)]
    Config {},

    #[returns(Option<super::asset::AssetParams>)]
    AssetParams {
        denom: String,
    },

    #[returns(Vec<super::asset::AssetParams>)]
    AllAssetParams {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(super::vault::VaultConfig)]
    VaultConfig {
        /// Address of vault
        address: String,
    },

    #[returns(Vec<super::vault::VaultConfig>)]
    AllVaultConfigs {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(cw_paginate::PaginationResponse<super::vault::VaultConfig>)]
    AllVaultConfigsV2 {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(Decimal)]
    TargetHealthFactor {},

    /// Compute the total amount deposited of the given asset across Red Bank
    /// and Credit Manager.
    #[returns(TotalDepositResponse)]
    TotalDeposit {
        denom: String,
    },

    /// Compute the total amount deposited for paginated assets across Red Bank
    /// and Credit Manager.
    #[returns(cw_paginate::PaginationResponse<TotalDepositResponse>)]
    AllTotalDepositsV2 {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    /// Address provider returns addresses for all protocol contracts
    pub address_provider: String,
}

#[cw_serde]
pub struct TotalDepositResponse {
    pub denom: String,
    pub cap: Uint128,
    pub amount: Uint128,
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
