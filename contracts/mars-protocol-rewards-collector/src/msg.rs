use osmo_bindings::{OsmosisMsg, Step};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CosmosMsg, Decimal, Uint128};

use mars_outpost::asset::Asset;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub config: CreateOrUpdateConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CreateOrUpdateConfig {
    pub owner: Option<String>,
    pub address_provider_address: Option<String>,
    /// Percentage of fees that are sent to the safety fund
    pub safety_tax_rate: Option<Decimal>,
    /// The asset to which the safety fund share is converted
    pub safety_fund_asset: Option<Asset>,
    /// The asset to which the fee collector share is converted
    pub fee_collector_asset: Option<Asset>,
    /// The channel id of the mars hub
    pub channel_id: Option<String>,
    /// Revision, used to determine the IBC Block timeout
    pub revision: Option<u64>,
    /// Block timeout
    pub block_timeout: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Update contract config
    UpdateConfig {
        config: CreateOrUpdateConfig,
    },

    /// Withdraw maTokens from the red bank
    WithdrawFromRedBank {
        asset: Asset,
        amount: Option<Uint128>,
    },

    /// Swap any asset on the contract
    SwapAsset {
        asset_in: Asset,
        amount: Option<Uint128>,
        fee_collector_asset_steps: Vec<Step>,
        safety_fund_asset_steps: Vec<Step>,
    },

    /// Distribute the accrued protocol income between the safety fund and the fee modules on mars hub,
    /// according to the split set in config.
    /// Callable by any address.
    DistributeProtocolRewards {
        /// Asset market fees to distribute
        asset: Asset,
        /// Amount to distribute to protocol contracts, defaults to contract balance if not specified
        amount: Option<Uint128>,
    },

    /// Execute Cosmos msg (only callable by owner)
    ExecuteCosmosMsg(CosmosMsg<OsmosisMsg>),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get config parameters
    Config {},
}
