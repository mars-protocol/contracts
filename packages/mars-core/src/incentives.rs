use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};

use crate::math::decimal::Decimal;

/// Global configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Contract owner
    pub owner: Addr,
    /// Address provider returns addresses for all protocol contracts
    pub address_provider_address: Addr,
}

/// Incentive Metadata for a given incentive
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AssetIncentive {
    /// How much MARS per second is emitted to be then distributed to all maToken holders
    pub emission_per_second: Uint128,
    /// Total MARS assigned for distribution since the start of the incentive
    pub index: Decimal,
    /// Last time (in seconds) index was updated
    pub last_updated: u64,
}

/// Response to AssetIncentive query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AssetIncentiveResponse {
    /// Existing asset incentive for a given address. Will return None if it doesn't exist
    pub asset_incentive: Option<AssetIncentive>,
}

pub mod msg {
    use cosmwasm_std::{Addr, CosmosMsg, Uint128};
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    pub struct InstantiateMsg {
        /// Contract owner
        pub owner: String,
        /// Address provider returns addresses for all protocol contracts
        pub address_provider_address: String,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum ExecuteMsg {
        /// Set emission per second for an asset to holders of its maToken
        SetAssetIncentive {
            /// maToken address associated with the incentives
            ma_token_address: String,
            /// How many MARS will be assigned per second to be distributed among all maToken
            /// holders
            emission_per_second: Uint128,
        },

        /// Handle balance change updating user and asset rewards.
        /// Sent from an external contract, triggered on user balance changes.
        /// Will return an empty response if no incentive is applied for the asset
        BalanceChange {
            /// User address. Address is trusted as it must be validated by the maToken
            /// contract before calling this method
            user_address: Addr,
            /// User maToken balance up to the instant before the change
            user_balance_before: Uint128,
            /// Total maToken supply up to the instant before the change
            total_supply_before: Uint128,
        },

        /// Claim rewards. MARS rewards accrued by the user will be staked into xMARS before
        /// being sent.
        ClaimRewards {},

        /// Update contract config (only callable by owner)
        UpdateConfig {
            owner: Option<String>,
            address_provider_address: Option<String>,
        },

        /// Execute Cosmos msg (only callable by owner)
        ExecuteCosmosMsg(CosmosMsg),
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum QueryMsg {
        /// Query contract config
        Config {},

        /// Query info about asset incentive for a given maToken
        AssetIncentive { ma_token_address: String },

        /// Query user current unclaimed rewards
        UserUnclaimedRewards { user_address: String },
    }
}
