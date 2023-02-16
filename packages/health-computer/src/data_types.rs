use std::collections::HashMap;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use mars_red_bank_types::red_bank::Market;
use mars_rover::adapters::vault::{VaultConfig, VaultPositionValue};

/// Used as storage when trying to compute Health
#[cw_serde]
pub struct CollateralValue {
    pub total_collateral_value: Uint128,
    pub max_ltv_adjusted_collateral: Uint128,
    pub liquidation_threshold_adjusted_collateral: Uint128,
}

#[cw_serde]
#[derive(Default)]
pub struct DenomsData {
    /// Must include data from info.base token for the vaults
    pub prices: HashMap<String, Decimal>,
    pub markets: HashMap<String, Market>,
}

#[cw_serde]
#[derive(Default)]
pub struct VaultsData {
    /// explain this, unlocked or locked value
    /// given the pricing method of vaults, cannot use individual coins
    pub vault_values: HashMap<Addr, VaultPositionValue>,
    pub vault_configs: HashMap<Addr, VaultConfig>,
}
