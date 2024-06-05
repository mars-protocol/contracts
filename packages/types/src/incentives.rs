use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_paginate::PaginationResponse;
use mars_owner::OwnerUpdate;

use crate::credit_manager::ActionCoin;

/// Global configuration
#[cw_serde]
pub struct Config {
    /// Address provider
    pub address_provider: Addr,
    /// The maximum number of incentive denoms that can be whitelisted at any given time. This is
    /// a guard against accidentally whitelisting too many denoms, which could cause max gas errors.
    pub max_whitelisted_denoms: u8,
}

/// Incentive Metadata for a given incentive
#[cw_serde]
pub struct IncentiveState {
    /// An index that represents how many incentive tokens have been distributed per unit of collateral
    pub index: Decimal,
    /// Last time (in seconds) index was updated
    pub last_updated: u64,
}

/// Incentive Metadata for a given incentive denom
#[cw_serde]
pub struct IncentiveStateResponse {
    /// The denom for which users get the incentive if they provide collateral in the Red Bank
    pub collateral_denom: String,
    /// The denom of the token these incentives are paid with
    pub incentive_denom: String,
    /// An index that represents how many incentive tokens have been distributed per unit of collateral
    pub index: Decimal,
    /// Last time (in seconds) index was updated
    pub last_updated: u64,
}

impl IncentiveStateResponse {
    pub fn from(
        collateral_denom: impl Into<String>,
        incentive_denom: impl Into<String>,
        is: IncentiveState,
    ) -> Self {
        Self {
            collateral_denom: collateral_denom.into(),
            incentive_denom: incentive_denom.into(),
            index: is.index,
            last_updated: is.last_updated,
        }
    }
}

#[cw_serde]
pub struct WhitelistEntry {
    /// The incentive token denom that is whitelisted
    pub denom: String,
    /// The minimum emission rate per second for this incentive token
    pub min_emission_rate: Uint128,
}

impl From<&(&str, u128)> for WhitelistEntry {
    fn from((denom, min_emission_rate): &(&str, u128)) -> Self {
        Self {
            denom: denom.to_string(),
            min_emission_rate: Uint128::from(*min_emission_rate),
        }
    }
}

impl From<(String, Uint128)> for WhitelistEntry {
    fn from((denom, min_emission_rate): (String, Uint128)) -> Self {
        Self {
            denom,
            min_emission_rate,
        }
    }
}

#[cw_serde]
pub struct InstantiateMsg {
    /// Contract owner
    pub owner: String,
    /// Address provider
    pub address_provider: String,
    /// The amount of time in seconds for each incentive epoch. This is the minimum amount of time
    /// that an incentive can last, and each incentive must be a multiple of this duration.
    pub epoch_duration: u64,
    /// The maximum number of incentive denoms that can be whitelisted at any given time. This is
    /// a guard against accidentally whitelisting too many denoms, which could cause max gas errors.
    pub max_whitelisted_denoms: u8,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Add or remove incentive denoms from the whitelist. Only admin can do this.
    UpdateWhitelist {
        /// The denoms to add to the whitelist as well as a minimum emission rate per second for
        /// each. If the denom is already in the whitelist, the minimum emission rate will be updated.
        add_denoms: Vec<WhitelistEntry>,
        /// The denoms to remove from the whitelist. This will update the index of the incentive
        /// state and then remove any active incentive schedules.
        ///
        /// NB: If any incentive schedules are still active for this incentive denom, the incentive
        /// tokens will be trapped forever in the contract.
        remove_denoms: Vec<String>,
    },
    /// Add incentives for a given collateral denom and incentive denom pair
    SetAssetIncentive {
        /// The denom of the collatearal token to receive incentives
        collateral_denom: String,
        /// The denom of the token to give incentives with
        incentive_denom: String,
        /// How many `incentive_denom` tokens will be assigned per second to be distributed among
        /// all Red Bank depositors
        emission_per_second: Uint128,
        /// Start time of the incentive (in seconds) since the UNIX epoch (00:00:00 on 1970-01-01 UTC).
        start_time: u64,
        /// How many seconds the incentives last
        duration: u64,
    },

    /// Handle balance change updating user and asset rewards.
    /// Sent from an external contract, triggered on user balance changes.
    /// Will return an empty response if no incentive is applied for the asset
    BalanceChange {
        /// User address. Address is trusted as it must be validated by the Red Bank
        /// contract before calling this method
        user_addr: Addr,
        /// Credit account id (Rover)
        account_id: Option<String>,
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
        /// Credit account id (Rover)
        account_id: Option<String>,
        /// Start pagination after this collateral denom
        start_after_collateral_denom: Option<String>,
        /// Start pagination after this incentive denom. If supplied you must also supply
        /// start_after_collateral_denom.
        start_after_incentive_denom: Option<String>,
        /// The maximum number of results to return. If not set, 5 is used. If larger than 10,
        /// 10 is used.
        limit: Option<u32>,
    },

    ClaimStakedAstroLpRewards {
        account_id: String,
        lp_denom: String,
    },

    /// Stake Astroport LP tokens in astroport incentives contract to receive rewards.
    StakeAstroLp {
        /// User credit account Id
        account_id: String,
        /// AstroLp token to stake.
        lp_coin: Coin,
    },

    /// Unstake Astroport LP tokens from astroport incentives contract.
    /// Sends tokens back to the users credit account
    UnstakeAstroLp {
        /// User credit account Id
        account_id: String,
        /// AstroLp token to unstake.
        lp_coin: ActionCoin,
    },

    /// Update contract config (only callable by owner)
    UpdateConfig {
        /// The address provider contract address
        address_provider: Option<String>,
        /// The maximum number of incentive denoms that can be whitelisted at any given time. This is
        /// a guard against accidentally whitelisting too many denoms, which could cause max gas errors.
        max_whitelisted_denoms: Option<u8>,
    },

    /// Manages admin role state
    UpdateOwner(OwnerUpdate),

    // Manages migration. It is used to handle migration in batches to avoid out of gas errors.
    Migrate(MigrateV1ToV2),
}

/// Migrate from V1 to V2, only owner can call
#[cw_serde]
pub enum MigrateV1ToV2 {
    /// Migrate users indexes and unclaimed rewards in batches
    UsersIndexesAndRewards {
        limit: u32,
    },
    /// Clears old V1 state once all batches are migrated or after a certain time
    ClearV1State {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Query account staked LP rewards
    #[returns(PaginatedLpRewardsResponse)]
    StakedAstroLpRewards {
        /// The id of the account who owns the LP
        account_id: String,
        /// Denom of LP that is accruing rewards
        lp_denom: String,
    },

    /// Query all active incentive emissions for a collateral denom
    #[returns(Vec<ActiveEmission>)]
    ActiveEmissions {
        /// The denom of the token that users supply as collateral to receive incentives
        collateral_denom: String,
    },

    /// Query contract config
    #[returns(ConfigResponse)]
    Config {},

    /// Query info about the state of an incentive for a given collateral and incentive denom pair
    #[returns(IncentiveStateResponse)]
    IncentiveState {
        /// The denom of the token that users supply as collateral to receive incentives
        collateral_denom: String,
        /// The denom of the token which is used to give incentives with
        incentive_denom: String,
    },

    /// Enumerate incentive states with pagination
    #[returns(Vec<IncentiveStateResponse>)]
    IncentiveStates {
        /// Start pagination after this collateral denom
        start_after_collateral_denom: Option<String>,
        /// Start pagination after this incentive denom. If supplied you must also supply
        /// start_after_collateral_denom.
        start_after_incentive_denom: Option<String>,
        /// The maximum number of results to return. If not set, 5 is used. If larger than 10,
        /// 10 is used.
        limit: Option<u32>,
    },

    /// Queries the planned emission rate for a given collateral and incentive denom tuple at the
    /// specified unix timestamp. The emission rate returned is the amount of incentive tokens
    /// that will be emitted per second for each unit of collateral supplied during the epoch.
    /// NB: that the returned value can change if someone adds incentives to the contract.
    #[returns(Uint128)]
    Emission {
        /// The denom of the token that users supply as collateral to receive incentives
        collateral_denom: String,
        /// The denom of the token which is used to give incentives with
        incentive_denom: String,
        /// The unix timestamp in second to query the emission rate at.
        timestamp: u64,
    },

    /// Enumerate all incentive emission rates with pagination for a specified collateral and
    /// indentive denom pair
    #[returns(Vec<EmissionResponse>)]
    Emissions {
        /// The denom of the token that users supply as collateral to receive incentives
        collateral_denom: String,
        /// The denom of the token which is used to give incentives with
        incentive_denom: String,
        /// Start pagination after this timestamp
        start_after_timestamp: Option<u64>,
        /// The maximum number of results to return. If not set, 5 is used. If larger than 10,
        /// 10 is used.
        limit: Option<u32>,
    },

    /// Enumerate a users LP positions with pagination
    #[returns(PaginatedStakedLpResponse)]
    StakedAstroLpPositions {
        /// The id of the account who owns the LP
        account_id: String,
        /// Start pagination after this lp denom, if used.
        start_after: Option<String>,
        /// The maximum number of results to return. If not set, 5 is used. If larger than 10,
        /// 10 is used.
        limit: Option<u32>,
    },

    /// Get specific details on a users LP Position
    #[returns(StakedLpPositionResponse)]
    StakedAstroLpPosition {
        /// The id of the account who owns the LP
        account_id: String,
        /// The denom of the LP position
        lp_denom: String,
    },

    /// Query user current unclaimed rewards
    #[returns(Vec<cosmwasm_std::Coin>)]
    UserUnclaimedRewards {
        /// The user address for which to query unclaimed rewards
        user: String,
        /// Credit account id (Rover)
        account_id: Option<String>,
        /// Start pagination after this collateral denom
        start_after_collateral_denom: Option<String>,
        /// Start pagination after this incentive denom. If supplied you must also supply
        /// start_after_collateral_denom.
        start_after_incentive_denom: Option<String>,
        /// The maximum number of results to return. If not set, 5 is used. If larger than 10,
        /// 10 is used.
        limit: Option<u32>,
    },

    /// Queries the incentive denom whitelist. Returns a Vec<(String, Uint128)> containing the
    /// denoms of all whitelisted incentive denoms, as well as the minimum emission rate for each.
    #[returns(Vec<WhitelistEntry>)]
    Whitelist {},
}

#[cw_serde]
pub struct EmissionResponse {
    /// The unix timestamp in seconds at which the emission epoch starts
    pub epoch_start: u64,
    /// The emission rate returned is the amount of incentive tokens that will be emitted per
    /// second for each unit of collateral supplied during the epoch.
    pub emission_rate: Uint128,
}

impl From<(u64, Uint128)> for EmissionResponse {
    fn from((epoch_start, emission_rate): (u64, Uint128)) -> Self {
        Self {
            epoch_start,
            emission_rate,
        }
    }
}

#[cw_serde]
/// The currently active emission for a given incentive denom
pub struct ActiveEmission {
    /// The denom for which incentives are being distributed
    pub denom: String,
    /// The amount of incentive tokens that are being emitted per second
    pub emission_rate: Uint128,
}

impl From<(String, Uint128)> for ActiveEmission {
    fn from((denom, emission_rate): (String, Uint128)) -> Self {
        Self {
            denom,
            emission_rate,
        }
    }
}

#[cw_serde]
pub struct ConfigResponse {
    /// The contract's owner
    pub owner: Option<String>,
    /// The contract's proposed owner
    pub proposed_new_owner: Option<String>,
    /// Address provider
    pub address_provider: Addr,
    /// The maximum number of incentive denoms that can be whitelisted at any given time. This is
    /// a guard against accidentally whitelisting too many denoms, which could cause max gas errors.
    pub max_whitelisted_denoms: u8,
    /// The epoch duration in seconds
    pub epoch_duration: u64,
    /// The count of the number of whitelisted incentive denoms
    pub whitelist_count: u8,
}

#[cw_serde]
pub struct StakedLpPositionResponse {
    pub lp_coin: Coin,
    pub rewards: Vec<Coin>,
}

pub type PaginatedStakedLpResponse = PaginationResponse<StakedLpPositionResponse>;
pub type PaginatedLpRewardsResponse = PaginationResponse<(String, Vec<Coin>)>;
#[cw_serde]
pub enum LpModification {
    Deposit,
    Withdraw,
}

impl From<LpModification> for String {
    fn from(lp_modification: LpModification) -> Self {
        match lp_modification {
            LpModification::Deposit => "Deposit".to_string(),
            LpModification::Withdraw => "Withdraw".to_string(),
        }
    }
}
