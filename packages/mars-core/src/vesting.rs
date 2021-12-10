use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Api, StdResult, Uint128};
use cw20::Cw20ReceiveMsg;

// T = String (unchecked) or Addr (checked)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config<T> {
    /// Address provider address
    pub address_provider_address: T,
    /// UNIX timestamp, in seconds, of when unlocking is to be started
    pub unlock_start_time: u64,
    /// Number of seconds during which no token will be unlocked
    pub unlock_cliff: u64,
    /// Number of seconds taken for tokens to be fully unlocked
    pub unlock_duration: u64,
}

impl From<Config<Addr>> for Config<String> {
    fn from(config: Config<Addr>) -> Self {
        Config {
            address_provider_address: config.address_provider_address.to_string(),
            unlock_start_time: config.unlock_start_time,
            unlock_cliff: config.unlock_cliff,
            unlock_duration: config.unlock_duration,
        }
    }
}

impl Config<String> {
    pub fn check(&self, api: &dyn Api) -> StdResult<Config<Addr>> {
        Ok(Config {
            address_provider_address: api.addr_validate(&self.address_provider_address)?,
            unlock_start_time: self.unlock_start_time,
            unlock_cliff: self.unlock_cliff,
            unlock_duration: self.unlock_duration,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Allocation {
    /// Total amount of MARS allocated
    pub mars_allocated_amount: Uint128,
    /// Amount of MARS already withdrawn
    pub mars_withdrawn_amount: Uint128,
    /// Amount of MARS staked in staking contract
    pub mars_staked_amount: Uint128,
    /// Amount of xMARS received by staking MARS tokens in staking contract
    pub xmars_minted_amount: Uint128,
}

/// Snapshot of a recipient's xMARS amount. Used to calculate the recipient's voting power when
/// voting for a governance proposal.
///
/// The first number is block height. The second number is the amount of xMARS the recipient has at
/// this block height.
pub type Snapshot = (u64, Uint128);

pub mod msg {
    use super::*;

    pub type InstantiateMsg = Config<String>;

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum ExecuteMsg {
        /// Implementation of cw20 receive msg
        Receive(Cw20ReceiveMsg),
        /// Withdraw unlocked MARS token
        Withdraw {},
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum ReceiveMsg {
        /// Create a new allocation for a recipeint
        CreateAllocation { user_address: String },
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum QueryMsg {
        /// Config of this contract. Returns `Config<String>`
        Config {},
        /// Status of an allocation. Returns `Allocation`
        Allocation { user_address: String },
        /// Total amount of xMARS owned by a recipient at a certain height
        VotingPowerAt { user_address: String, block: u64 },
    }
}
