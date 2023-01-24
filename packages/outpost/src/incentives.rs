use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use mars_owner::OwnerUpdate;

/// Global configuration
#[cw_serde]
pub struct Config {
    /// Address provider
    pub address_provider: Addr,
    /// Mars Token Denom
    pub mars_denom: String,
}

/// Incentive Metadata for a given incentive
#[cw_serde]
pub struct AssetIncentive {
    /// How much MARS per second is emitted to be then distributed to all Red Bank depositors
    pub emission_per_second: Uint128,
    /// Start time for the incentive
    pub start_time: u64,
    /// How many seconds the incentives last
    pub duration: u64,
    /// Total MARS assigned for distribution since the start of the incentive
    pub index: Decimal,
    /// Last time (in seconds) index was updated
    pub last_updated: u64,
}

/// Response to AssetIncentive query
#[cw_serde]
pub struct AssetIncentiveResponse {
    /// Existing asset incentive for a given address. Will return None if it doesn't exist
    pub asset_incentive: Option<AssetIncentive>,
}

#[cw_serde]
pub struct InstantiateMsg {
    /// Contract owner
    pub owner: String,
    /// Address provider
    pub address_provider: String,
    /// Mars token denom
    pub mars_denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Set incentive params for an asset to its depositor at Red Bank.
    ///
    /// If there is no incentive for the asset, all params are required.
    /// New incentive can be set (rescheduled) if current one has finished (current_block_time > start_time + duration).
    SetAssetIncentive {
        /// Asset denom associated with the incentives
        denom: String,
        /// How many MARS will be assigned per second to be distributed among all Red Bank
        /// depositors
        emission_per_second: Option<Uint128>,
        /// Start time for the incentive
        start_time: Option<u64>,
        /// How many seconds the incentives last
        duration: Option<u64>,
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
        address_provider: Option<String>,
        mars_denom: Option<String>,
    },

    /// Manages admin role state
    UpdateOwner(OwnerUpdate),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Query contract config
    #[returns(ConfigResponse)]
    Config {},

    /// Query info about asset incentive for a given denom
    #[returns(AssetIncentiveResponse)]
    AssetIncentive {
        denom: String,
    },

    /// Query user current unclaimed rewards
    #[returns(Uint128)]
    UserUnclaimedRewards {
        user: String,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    /// The contract's owner
    pub owner: Option<String>,
    /// The contract's proposed owner
    pub proposed_new_owner: Option<String>,
    /// Address provider
    pub address_provider: Addr,
    /// Mars Token Denom
    pub mars_denom: String,
}
