use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, CosmosMsg, Decimal, Uint128};

/// Global configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    /// Contract owner
    pub owner: Addr,
    /// Address provider
    pub address_provider: Addr,
    /// Mars Token Denom
    pub mars_denom: String,
}

/// Incentive Metadata for a given incentive
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct AssetIncentive {
    /// How much MARS per second is emitted to be then distributed to all Red Bank depositors
    pub emission_per_second: Uint128,
    /// Total MARS assigned for distribution since the start of the incentive
    pub index: Decimal,
    /// Last time (in seconds) index was updated
    pub last_updated: u64,
}

/// Response to AssetIncentive query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct AssetIncentiveResponse {
    /// Existing asset incentive for a given address. Will return None if it doesn't exist
    pub asset_incentive: Option<AssetIncentive>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    /// Contract owner
    pub owner: String,
    /// Address provider
    pub address_provider: String,
    /// Mars token denom
    pub mars_denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Set emission per second for an asset to its depositor at Red Bank
    SetAssetIncentive {
        /// Asset denom associated with the incentives
        denom: String,
        /// How many MARS will be assigned per second to be distributed among all Red Bank
        /// depositors
        emission_per_second: Uint128,
    },

    /// Handle balance change updating user and asset rewards.
    /// Sent from an external contract, triggered on user balance changes.
    /// Will return an empty response if no incentive is applied for the asset
    BalanceChange {
        /// User address. Address is trusted as it must be validated by the Red Bank
        /// contract before calling this method
        user_addr: Addr,
        /// Denom of the asset of which deposited balance is changed
        denom: String,
        /// The user's scaled collateral amount up to the instant before the change
        user_amount_scaled_before: Uint128,
        /// The market's total scaled collateral amount up to the instant before the change
        total_amount_scaled_before: Uint128,
    },

    /// Claim rewards. MARS rewards accrued by the user will be staked into xMARS before
    /// being sent.
    ClaimRewards {},

    /// Update contract config (only callable by owner)
    UpdateConfig {
        owner: Option<String>,
        address_provider: Option<String>,
        mars_denom: Option<String>,
    },

    /// Execute Cosmos msg (only callable by owner)
    ExecuteCosmosMsg(CosmosMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Query contract config
    Config {},

    /// Query info about asset incentive for a given denom
    AssetIncentive {
        denom: String,
    },

    /// Query user current unclaimed rewards
    UserUnclaimedRewards {
        user: String,
    },
}
