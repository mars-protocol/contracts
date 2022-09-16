use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Partial compatibility with EIP-4626
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Enters list of `Vec<Coin>` into a vault strategy in exchange for vault tokens.
    Deposit {},
    /// Withdraw underlying coins in vault by exchanging vault `Coin`
    Withdraw {},
    /// A privileged action only to be used by Rover. Same as `Withdraw` except it bypasses any lockup period
    /// restrictions on the vault. Used only in the case position is unhealthy and requires immediate liquidation.
    ForceWithdraw {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Vault requirements, lockup, & vault token denom
    #[returns(VaultInfo)]
    Info {},
    /// All the coins that would be redeemed for in exchange for
    /// vault coins. Used by Rover to calculate vault position values.
    #[returns(Vec<Coin>)]
    PreviewRedeem { shares: Uint128 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
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
