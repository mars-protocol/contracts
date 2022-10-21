use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal};

use crate::adapters::swap::SwapperUnchecked;
use crate::adapters::{OracleUnchecked, RedBankUnchecked, VaultUnchecked};

#[cw_serde]
pub struct InstantiateMsg {
    /// The address with privileged access to update config
    pub owner: String,
    /// Whitelisted coin denoms approved by governance
    pub allowed_coins: Vec<String>,
    /// Whitelisted vaults approved by governance that implement credit manager's vault interface
    /// Includes a deposit cap that enforces a TLV limit for risk mitigation
    pub allowed_vaults: Vec<VaultInstantiateConfig>,
    /// The Mars Protocol money market contract where we borrow assets from
    pub red_bank: RedBankUnchecked,
    /// The Mars Protocol oracle contract. We read prices of assets here.
    pub oracle: OracleUnchecked,
    /// The maximum percent a liquidator can profit from a liquidation action
    pub max_liquidation_bonus: Decimal,
    /// The maximum percent a liquidator can decrease the debt amount of the liquidatee
    pub max_close_factor: Decimal,
    /// Helper contract for making swaps
    pub swapper: SwapperUnchecked,
}

#[cw_serde]
pub struct VaultInstantiateConfig {
    pub vault: VaultUnchecked,
    pub deposit_cap: Coin,
}

/// Used when you want to update fields on Instantiate config
#[cw_serde]
#[derive(Default)]
pub struct ConfigUpdates {
    pub account_nft: Option<String>,
    pub owner: Option<String>,
    pub allowed_coins: Option<Vec<String>>,
    pub allowed_vaults: Option<Vec<VaultUnchecked>>,
    pub red_bank: Option<RedBankUnchecked>,
    pub oracle: Option<OracleUnchecked>,
    pub max_liquidation_bonus: Option<Decimal>,
    pub max_close_factor: Option<Decimal>,
    pub swapper: Option<SwapperUnchecked>,
}
