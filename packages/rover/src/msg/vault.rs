use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Timestamp, Uint128};

pub const UNLOCKING_POSITION_CREATED_EVENT_TYPE: &str = "unlocking_position_created";
pub const UNLOCKING_POSITION_ATTR: &str = "id";

/// Partial compatibility with EIP-4626
#[cw_serde]
pub enum ExecuteMsg {
    /// Enters list of `Vec<Coin>` into a vault strategy in exchange for vault tokens.
    Deposit {},
    /// Withdraw underlying coins in vault by exchanging vault `Coin`
    Withdraw {},
    /// A privileged action only to be used by Rover. Same as `Withdraw` except it bypasses any lockup period
    /// restrictions on the vault. Used only in the case position is unhealthy and requires immediate liquidation.
    ForceWithdraw {},
    /// Some vaults have lockup periods (typically between 1-14 days). This action sends vault `Coin`
    /// which is locked for vault lockup period and available to `Unlock` after that time has elapsed.
    /// On response, vault sends back `unlocking_position_created` event with attribute `id` representing
    /// the new unlocking coins position.
    RequestUnlock {},
    /// Withdraw assets in vault that have been unlocked for given unlocking position
    WithdrawUnlocked { id: Uint128 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Vault requirements, lockup, & vault token denom
    #[returns(VaultInfo)]
    Info {},
    /// All the coins that would be redeemed for in exchange for
    /// vault coins. Used by Rover to calculate vault position values.
    #[returns(Vec<cosmwasm_std::Coin>)]
    PreviewRedeem { amount: Uint128 },
    /// Returns the total vault coins issued. In order to prevent Cream-attack, we cannot
    /// query the bank module for this amount.
    #[returns(Uint128)]
    TotalVaultCoinsIssued {},
    /// The vault positions that this address has requested to unlock
    #[returns(Vec<UnlockingPosition>)]
    UnlockingPositionsForAddr { addr: String },
    #[returns(UnlockingPosition)]
    UnlockingPosition { id: Uint128 },
}

#[cw_serde]
pub struct VaultInfo {
    /// Denom of vault token
    pub vault_coin_denom: String,
    /// Coin denoms required to enter vault.
    /// Multiple vectors indicate the vault accepts more than one combination to enter.
    pub accepts: Vec<Vec<String>>,
    /// Time in seconds for unlock period
    pub lockup: Option<u64>,
}

#[cw_serde]
pub struct UnlockingPosition {
    /// Unique identifier representing the unlocking position. Needed for `ExecuteMsg::Unlock {}` call.
    pub id: Uint128,
    /// Number of vault tokens
    pub amount: Uint128,
    /// Absolute time when position unlocks in seconds since the UNIX epoch (00:00:00 on 1970-01-01 UTC)
    pub unlocked_at: Timestamp,
}
