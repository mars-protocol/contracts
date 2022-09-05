use cosmwasm_std::{Coin, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Partial compatibility with EIP-4626
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Enters list of `Vec<Coin>` into a vault strategy in exchange for vault tokens.
    Deposit,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns `VaultInfo` representing vault requirements, lockup, & vault token denom
    Info,
    /// Returns `Vec<Coin>` representing all the coins that would be redeemed for in exchange for
    /// vault coins. Used by Rover to calculate vault position values.
    PreviewRedeem { shares: Uint128 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct VaultInfo {
    /// Coins required to enter vault.
    /// Amount will be proportional to the share of which it should occupy in the group
    /// (e.g. { denom: osmo, amount: 1 }, { denom: atom, amount: 1 } indicate a 50-50 split)  
    pub coins: Vec<Coin>,
    /// Time in seconds for unlock period
    pub lockup: Option<u64>,
    /// Denom of vault token
    pub token_denom: String,
}
