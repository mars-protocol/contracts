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
    /// How many incentive tokens per second is emitted to be then distributed to all Red Bank depositors
    pub emission_per_second: Uint128,
    /// Start time of the incentive (in seconds) since the UNIX epoch (00:00:00 on 1970-01-01 UTC)
    pub start_time: u64,
    /// How many seconds the incentives last
    pub duration: u64,
    /// Total incentive tokens assigned for distribution since the start of the incentive
    pub index: Decimal,
    /// Last time (in seconds) index was updated
    pub last_updated: u64,
}

/// Incentive Metadata for a given incentive denom
#[cw_serde]
pub struct AssetIncentiveResponse {
    /// The denom for which users get the incentive if they provide collateral in the Red Bank
    pub collateral_denom: String,
    /// The denom of the token these incentives are paid with
    pub incentive_denom: String,
    /// How many incentive tokens per second is emitted to be then distributed to all Red Bank depositors
    pub emission_per_second: Uint128,
    /// Start time of the incentive (in seconds) since the UNIX epoch (00:00:00 on 1970-01-01 UTC)
    pub start_time: u64,
    /// How many seconds the incentives last
    pub duration: u64,
    /// Total incentive tokens assigned for distribution since the start of the incentive
    pub index: Decimal,
    /// Last time (in seconds) index was updated
    pub last_updated: u64,
}

impl AssetIncentiveResponse {
    pub fn from(
        collateral_denom: impl Into<String>,
        incentive_denom: impl Into<String>,
        ai: AssetIncentive,
    ) -> Self {
        Self {
            collateral_denom: collateral_denom.into(),
            incentive_denom: incentive_denom.into(),
            emission_per_second: ai.emission_per_second,
            start_time: ai.start_time,
            duration: ai.duration,
            index: ai.index,
            last_updated: ai.last_updated,
        }
    }
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
    /// Add or remove incentive denoms from the whitelist. Only admin can do this.
    UpdateWhitelist {
        /// The denoms to add to the whitelist
        add_denoms: Vec<String>,
        /// The denoms to remove from the whitelist
        remove_denoms: Vec<String>,
    },
    /// Set incentive params for an asset to its depositor at Red Bank.
    ///
    /// If there is no incentive for the asset, all params are required.
    /// New incentive can be set (rescheduled) if current one has finished (current_block_time > start_time + duration).
    SetAssetIncentive {
        /// The denom of the collatearal token to receive incentives
        collateral_denom: String,
        /// The denom of the token to give incentives with
        incentive_denom: String,
        /// How many `incentive_denom` tokens will be assigned per second to be distributed among
        /// all Red Bank depositors
        emission_per_second: Option<Uint128>,
        /// Start time of the incentive (in seconds) since the UNIX epoch (00:00:00 on 1970-01-01 UTC).
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
    ClaimRewards {
        /// Start pagination after this collateral denom
        start_after_collateral_denom: Option<String>,
        /// Start pagination after this incentive denom. If supplied you must also supply
        /// start_after_collateral_denom.
        start_after_incentive_denom: Option<String>,
        /// The maximum number of results to return. If not set, 5 is used. If larger than 10,
        /// 10 is used.
        limit: Option<u32>,
    },

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

    /// Query info about asset incentive for a given collateral and incentive denom pair
    #[returns(AssetIncentiveResponse)]
    AssetIncentive {
        /// The denom of the token that users supply as collateral to receive incentives
        collateral_denom: String,
        /// The denom of the token which is used to give incentives with
        incentive_denom: String,
    },

    /// Enumerate asset incentives with pagination
    #[returns(Vec<AssetIncentiveResponse>)]
    AssetIncentives {
        /// Start pagination after this collateral denom
        start_after_collateral_denom: Option<String>,
        /// Start pagination after this incentive denom. If supplied you must also supply
        /// start_after_collateral_denom.
        start_after_incentive_denom: Option<String>,
        /// The maximum number of results to return. If not set, 5 is used. If larger than 10,
        /// 10 is used.
        limit: Option<u32>,
    },

    /// Query user current unclaimed rewards
    #[returns(Uint128)]
    UserUnclaimedRewards {
        /// The user address for which to query unclaimed rewards
        user: String,
        /// Start pagination after this collateral denom
        start_after_collateral_denom: Option<String>,
        /// Start pagination after this incentive denom. If supplied you must also supply
        /// start_after_collateral_denom.
        start_after_incentive_denom: Option<String>,
        /// The maximum number of results to return. If not set, 5 is used. If larger than 10,
        /// 10 is used.
        limit: Option<u32>,
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
