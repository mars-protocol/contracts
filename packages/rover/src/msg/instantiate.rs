use crate::adapters::swap::SwapperUnchecked;
use crate::adapters::vault::VaultConfig;
use crate::adapters::vault::VaultUnchecked;
use crate::adapters::ZapperUnchecked;
use crate::adapters::{OracleUnchecked, RedBankUnchecked};
use crate::traits::Stringify;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    /// The address with privileged access to update config
    pub admin: String,
    /// Whitelisted coin denoms approved by governance
    pub allowed_coins: Vec<String>,
    /// Whitelisted vaults approved by governance that implement credit manager's vault interface
    /// Includes a deposit cap that enforces a TLV limit for risk mitigation
    pub allowed_vaults: Vec<VaultInstantiateConfig>,
    /// The Mars Protocol money market contract where we borrow assets from
    pub red_bank: RedBankUnchecked,
    /// The Mars Protocol oracle contract. We read prices of assets here.
    pub oracle: OracleUnchecked,
    /// The maximum percent a liquidator can decrease the debt amount of the liquidatee
    pub max_close_factor: Decimal,
    /// The maximum number of unlocking positions an account can have simultaneously
    /// Note: As health checking requires looping through each, this number must not be too large.
    ///       If so, having too many could prevent the account from being liquidated due to gas constraints.
    pub max_unlocking_positions: Uint128,
    /// Helper contract for making swaps
    pub swapper: SwapperUnchecked,
    /// Helper contract for adding/removing liquidity
    pub zapper: ZapperUnchecked,
}

#[cw_serde]
pub struct VaultInstantiateConfig {
    pub vault: VaultUnchecked,
    pub config: VaultConfig,
}

impl Stringify for Vec<VaultInstantiateConfig> {
    fn to_string(&self) -> String {
        self.iter()
            .map(|v| {
                format!(
                    "addr: {}, deposit_cap: {}, max_ltv: {}, liquidation_threshold: {}, whitelisted: {}",
                    v.vault.address,
                    v.config.deposit_cap,
                    v.config.max_ltv,
                    v.config.liquidation_threshold,
                    v.config.whitelisted
                )
            })
            .collect::<Vec<String>>()
            .join(" :: ")
    }
}

/// Used when you want to update fields on Instantiate config
#[cw_serde]
#[derive(Default)]
pub struct ConfigUpdates {
    pub account_nft: Option<String>,
    pub admin: Option<String>,
    pub allowed_coins: Option<Vec<String>>,
    pub vault_configs: Option<Vec<VaultInstantiateConfig>>,
    pub oracle: Option<OracleUnchecked>,
    pub max_close_factor: Option<Decimal>,
    pub max_unlocking_positions: Option<Uint128>,
    pub swapper: Option<SwapperUnchecked>,
    pub zapper: Option<ZapperUnchecked>,
}
