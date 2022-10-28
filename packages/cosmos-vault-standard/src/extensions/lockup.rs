use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Uint128};
use cw_utils::{Duration, Expiration};

#[cfg(feature = "cw20")]
use cw20::Cw20Coin;

/// Type for the unlocking position created event emitted on call to `Unlock`.
pub const UNLOCKING_POSITION_CREATED_EVENT_TYPE: &str = "unlocking_position_created";
/// Key for the lockup id attribute in the "unlocking position created" event that
/// is emitted on call to `Unlock`.
pub const UNLOCKING_POSITION_ATTR_KEY: &str = "lockup_id";

#[cw_serde]
pub enum LockupExecuteMsg {
    /// Unlock is called to initiate unlocking a locked position held by the
    /// vault.
    /// The caller must pass the native vault tokens in the funds field.
    /// Emits an event with type `UNLOCK_EVENT_TYPE` with an attribute with key
    /// `UNLOCKING_POSITION_ATTR_KEY` containing an u64 lockup_id.
    /// Also encodes the u64 lockup ID as binary and returns it in the Response's
    /// data field, so that it can be read by SubMsg replies.
    ///
    /// Like Redeem, this takes an amount so that the same API can be used for
    /// CW4626 and native tokens.
    Unlock { amount: Uint128 },

    /// Withdraw an unlocking position that has finished unlocking.
    WithdrawUnlocked {
        /// An optional field containing which address should receive the
        /// withdrawn underlying assets. If not set, the caller address will be
        /// used instead.
        recipient: Option<String>,
        /// The ID of the expired lockup to withdraw from.
        /// If None is passed, the vault will attempt to withdraw all expired
        /// lockup positions. Note that this can fail if there are too many
        /// lockup positions and the `max_contract_gas` limit is hit.
        lockup_id: u64,
    },

    /// Can be called by whitelisted addresses to bypass the lockup and
    /// immediately return the underlying assets. Used in the event of
    /// liquidation. The caller must pass the native vault tokens in the funds
    /// field.
    ForceWithdraw {
        /// The address which should receive the withdrawn assets. If not set,
        /// the caller address will be used instead.
        recipient: Option<String>,
        /// The amount of vault tokens to force unlock.
        amount: Uint128,
    },

    /// Force withdraw from a position that is already unlocking (Unlock has
    /// already been called).
    ForceWithdrawUnlocking {
        /// The ID of the unlocking position from which to force withdraw
        lockup_id: u64,
        /// Optional amounts of each underlying asset to be force withdrawn.
        /// If None is passed, the entire position will be force withdrawn.
        /// Vaults MAY require the ratio of assets to be the same as the ratio
        /// in the `deposit_assets` field returned by the `VaultInfo` query.
        amount: Option<Uint128>,
        #[cfg(feature = "cw20")]
        cw20s_amounts: Option<Vec<Cw20Coin>>,
        /// The address which should receive the withdrawn assets. If not set,
        /// the assets will be sent to the caller.
        recipient: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum LockupQueryMsg {
    /// Returns a `Vec<Lockup>` containing all the currently unclaimed lockup
    /// positions for the `owner`.
    #[returns(Vec<Lockup>)]
    Lockups {
        /// The address of the owner of the lockup
        owner: String,
        /// Return results only after this lockup_id
        start_after: Option<u64>,
        /// Max amount of results to return
        limit: Option<u32>,
    },

    /// Returns `Lockup` info about a specific lockup, by owner and ID.
    #[returns(Lockup)]
    Lockup { lockup_id: u64 },

    /// Returns `cw_utils::Duration` duration of the lockup.
    #[returns(Duration)]
    LockupDuration {},
}

/// Info about a currently unlocking position.
#[cw_serde]
pub struct Lockup {
    pub owner: Addr,
    pub id: u64,
    pub release_at: Expiration,
    pub coin: Coin,
    #[cfg(feature = "cw20")]
    pub cw20s: Vec<Cw20Coin>,
}
