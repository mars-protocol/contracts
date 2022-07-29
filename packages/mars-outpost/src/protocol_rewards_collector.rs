use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Api, CosmosMsg, Decimal, StdResult, Uint128};

use crate::error::MarsError;
use crate::helpers::decimal_param_le_one;

/// Global configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config<T> {
    /// Contract owner
    pub owner: T,
    /// Address provider returns addresses for all protocol contracts
    pub address_provider: T,
    /// Percentage of fees that are sent to the safety fund
    pub safety_tax_rate: Decimal,
    /// The asset to which the safety fund share is converted
    pub safety_fund_denom: String,
    /// The asset to which the fee collector share is converted
    pub fee_collector_denom: String,
    /// The channel ID of the mars hub
    pub channel_id: String,
    /// Revision number of Mars Hub's IBC client
    pub timeout_revision: u64,
    /// Number of blocks after which an IBC transfer is to be considered failed, if no acknowledgement is received
    pub timeout_blocks: u64,
    /// Number of seconds after which an IBC transfer is to be considered failed, if no acknowledgement is received
    pub timeout_seconds: u64,
}

impl<T> Config<T> {
    pub fn validate(&self) -> Result<(), MarsError> {
        decimal_param_le_one(self.safety_tax_rate, "safety_tax_rate")?;
        Ok(())
    }
}

impl Config<String> {
    pub fn check(&self, api: &dyn Api) -> StdResult<Config<Addr>> {
        Ok(Config {
            owner: api.addr_validate(&self.owner)?,
            address_provider: api.addr_validate(&self.address_provider)?,
            safety_tax_rate: self.safety_tax_rate,
            safety_fund_denom: self.safety_fund_denom.clone(),
            fee_collector_denom: self.fee_collector_denom.clone(),
            channel_id: self.channel_id.clone(),
            timeout_revision: self.timeout_revision,
            timeout_blocks: self.timeout_blocks,
            timeout_seconds: self.timeout_seconds,
        })
    }
}

impl From<Config<Addr>> for Config<String> {
    fn from(cfg: Config<Addr>) -> Self {
        Self {
            owner: cfg.owner.into(),
            address_provider: cfg.address_provider.into(),
            safety_tax_rate: cfg.safety_tax_rate,
            safety_fund_denom: cfg.safety_fund_denom.clone(),
            fee_collector_denom: cfg.fee_collector_denom.clone(),
            channel_id: cfg.channel_id.clone(),
            timeout_revision: cfg.timeout_revision,
            timeout_blocks: cfg.timeout_blocks,
            timeout_seconds: cfg.timeout_seconds,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, JsonSchema)]
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
    /// Revision number of Mars Hub's IBC light client
    pub timeout_revision: Option<u64>,
    /// Number of blocks after which an IBC transfer is to be considered failed, if no acknowledgement is received
    pub timeout_blocks: Option<u64>,
    /// Number of seconds after which an IBC transfer is to be considered failed, if no acknowledgement is received
    pub timeout_seconds: Option<u64>,
}

pub type InstantiateMsg = Config<String>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg<SwapInstruction, CustomMsg> {
    /// Update contract config
    UpdateConfig(CreateOrUpdateConfig),

    /// Configure the instruction for swapping an asset
    ///
    /// This is chain-specific, and can include parameters such as slippage tolerance and the routes
    /// for multi-step swaps
    SetInstruction {
        denom_in: String,
        denom_out: String,
        instruction: SwapInstruction,
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
    /// Get config parameters; response: `Config<String>`
    Config {},
    /// Get instruction for swapping an input denom into an output denom; response: `InstructionResponse`
    Instruction {
        denom_in: String,
        denom_out: String,
    },
    /// Enumerate all swap instructions; response: `Vec<InstructionResponse>`
    Instructions {
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstructionResponse<SwapInstruction> {
    pub denom_in: String,
    pub denom_out: String,
    pub instruction: SwapInstruction,
}
