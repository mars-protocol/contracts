use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, CosmosMsg, Decimal, Uint128};

use crate::error::MarsError;
use crate::helpers::decimal_param_le_one;

/// Global configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Contract owner
    pub owner_addr: Addr,
    /// Address provider returns addresses for all protocol contracts
    pub address_provider_addr: Addr,
    /// Percentage of fees that are sent to the safety fund
    pub safety_tax_rate: Decimal,
    /// The asset to which the safety fund share is converted
    pub safety_fund_denom: String,
    /// The asset to which the fee collector share is converted
    pub fee_collector_denom: String,
    /// The channel ID of the mars hub
    pub channel_id: String,
    /// revision, needed for the IBC block timeout
    /// TODO check where to find the revision
    pub revision: u64,
    /// Block timeout, when the IBC transfer times out
    pub block_timeout: u64,
}

impl Config {
    pub fn validate(&self) -> Result<(), MarsError> {
        decimal_param_le_one(self.safety_tax_rate, "safety_tax_rate")?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CreateOrUpdateConfig {
    /// Contract owner
    pub owner: Option<String>,
    /// Address provider returns addresses for all protocol contracts
    pub address_provider: Option<String>,
    /// Percentage of fees that are sent to the safety fund
    pub safety_tax_rate: Option<Decimal>,
    /// The asset to which the safety fund share is converted
    pub safety_fund_denom: Option<String>,
    /// The asset to which the fee collector share is converted
    pub fee_collector_denom: Option<String>,
    /// The channel id of the mars hub
    pub channel_id: Option<String>,
    /// Revision, used to determine the IBC Block timeout
    pub revision: Option<u64>,
    /// Block timeout
    pub block_timeout: Option<u64>,
}

pub type InstantiateMsg = Config;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg<SwapInstructions, CustomMsg> {
    /// Update contract config
    UpdateConfig(CreateOrUpdateConfig),

    /// Configure the instructions for swapping an asset
    ///
    /// This is chain-specific, and can include parameters such as slippage tolerance and the routes
    /// for multi-step swaps
    SetSwapInstructions {
        denom_in: String,
        denom_out: String,
        instructions: SwapInstructions,
    },

    /// Withdraw maTokens from the red bank
    WithdrawFromRedBank {
        denom: String,
        amount: Option<Uint128>,
    },

    /// Distribute the accrued protocol income between the safety fund and the fee modules on mars hub,
    /// according to the split set in config.
    /// Callable by any address.
    DistributeRewards {
        denom: String,
        amount: Option<Uint128>,
    },

    /// Swap any asset on the contract
    SwapAsset {
        denom: String,
        amount: Option<Uint128>,
    },

    /// Execute Cosmos msg (only callable by owner)
    ExecuteCosmosMsg(CosmosMsg<CustomMsg>),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get config parameters; response: `Config`
    Config {},
}
