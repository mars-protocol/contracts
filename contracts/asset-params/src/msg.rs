use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal};
use mars_owner::OwnerUpdate;
use mars_utils::helpers::decimal_param_le_one;
use crate::error::ValidationError;
use crate::types::{AssetParams, VaultConfigs, ConfigResponse};

#[cw_serde]
pub struct InstantiateMsg {
    /// Contract's owner
    pub owner: String,
    /// Contract's emergency owner
    pub emergency_owner: String,
    /// The maximum percent a liquidator can decrease the debt amount of the liquidatee
    pub close_factor: Decimal,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Manages owner state (only owner can call)
    UpdateOwner(OwnerUpdate),
    /// Update outpost's universal close factor (only owner can call)
    UpdateCloseFactor {
        close_factor: Decimal,
    },
    /// Initialize an asset on the money market (only owner can call)
    InitAsset {
        /// Asset related info
        denom: String,
        /// Asset parameters
        params: AssetParams,
    },
    /// Update an asset on the money market (only owner can call)
    UpdateAsset {
        /// Asset related info
        denom: String,
        /// Asset parameters
        params: AssetParams,
    },
    /// Init or Update the vault configs on Rover (only owner can call)
    InitOrUpdateVault {
        /// Vault related info
        address: Addr,
        /// Vault configuration
        config: VaultConfigs,
    }
}
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Config returns the owners and close factor
    #[returns(ConfigResponse)]
    Config {},
    /// Asset Params that are relevant to the Red Bank Markets and Rover
    #[returns(AssetParams)]
    AssetParamsResponse {
        denom: String,
    },
    /// Rover vault configs
    #[returns(VaultConfigs)]
    VaultConfigsResponse {
        address: Addr,
    },
}