use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::math::decimal::Decimal;
use cosmwasm_std::{Addr, Decimal as StdDecimal, Uint128};

/// Protocol configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Contract owner
    pub owner: Addr,

    /// Address provider address
    pub address_provider_address: Addr,

    /// Astroport factory contract address
    pub astroport_factory_address: Addr,
    /// Astroport max spread
    pub astroport_max_spread: StdDecimal,

    /// Cooldown duration in seconds
    pub cooldown_duration: u64,
}

/// Global State
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GlobalState {
    /// Total amount of Mars belonging to open claims
    pub total_mars_for_claimers: Uint128,
}

/// Unstaking cooldown data
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Claim {
    /// Block when the claim was created (Used to apply slash events when claiming)
    pub created_at_block: u64,
    /// Timestamp (in seconds) after which the claim is unlocked
    pub cooldown_end_timestamp: u64,
    /// Amount of Mars that the user is allowed to claim
    pub amount: Uint128,
}

/// Event where funds are taken from the Mars pool to cover a shortfall. The loss is covered
/// proportionally by all owners of the Mars pool (xMars holders and users with an open claim)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SlashEvent {
    /// Percentage of total Mars slashed
    pub slash_percentage: Decimal,
}

/// Response to Claim query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClaimResponse {
    /// Existing claim for a given address. Will return None if it doesn't exist
    pub claim: Option<Claim>,
}

pub mod msg {
    use cosmwasm_std::{Decimal as StdDecimal, Uint128};

    use cw20::Cw20ReceiveMsg;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    pub struct InstantiateMsg {
        pub config: CreateOrUpdateConfig,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    pub struct CreateOrUpdateConfig {
        pub owner: Option<String>,
        pub address_provider_address: Option<String>,
        pub astroport_factory_address: Option<String>,
        pub astroport_max_spread: Option<StdDecimal>,
        pub cooldown_duration: Option<u64>,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum ExecuteMsg {
        /// Update staking config
        UpdateConfig { config: CreateOrUpdateConfig },

        /// Implementation for cw20 receive msg
        Receive(Cw20ReceiveMsg),

        /// Close claim sending the claimable Mars to the specified address (sender is the default)
        Claim { recipient: Option<String> },

        /// Transfer Mars, deducting it proportionally from both xMars holders and addresses
        /// with an open claim
        TransferMars { amount: Uint128, recipient: String },

        /// Swap uusd on the contract to Mars. Meant for received protocol rewards in order
        /// for them to belong to xMars holders as underlying Mars.
        SwapUusdToMars { amount: Option<Uint128> },
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum ReceiveMsg {
        /// Stake Mars and mint xMars in return
        Stake {
            /// Address to receive the xMars tokens. Set to sender if not specified
            recipient: Option<String>,
        },

        /// Burn xMars and initiate a cooldown period on which the underlying Mars
        /// will be claimable. Only one open claim per address is allowed.
        Unstake {
            /// Address to claim the Mars tokens after cooldown. Set to sender is not specified
            recipient: Option<String>,
        },
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum QueryMsg {
        /// Get contract config
        Config {},
        /// Get contract global state
        GlobalState {},
        /// Compute the amount of xMars token to be minted by staking 1 unit of Mars token.
        /// The ratio may be undefined, in which case we return `Ok(None)`
        XMarsPerMars {},
        /// Compute the amount of Mars token to be claimed by burning 1 unit of xMars token.
        /// The ratio may be undefined, in which case we return `Ok(None)`
        MarsPerXMars {},
        /// Get open claim for given user. If claim exists, slash events are applied to the amount
        /// so actual amount of Mars received is given.
        Claim { user_address: String },
    }
}
