use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use cosmwasm_std::{Addr, Decimal};

use crate::asset::Asset;
use crate::error::MarsError;
use crate::helpers::decimal_param_le_one;

/// Global configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Contract owner
    pub owner: Addr,
    /// Address provider returns addresses for all protocol contracts
    pub address_provider_address: Addr,
    /// Percentage of fees that are sent to the safety fund
    pub safety_tax_rate: Decimal,
    /// The asset to which the safety fund share is converted
    pub safety_fund_asset: Asset,
    /// The asset to which the fee collector share is converted
    pub fee_collector_asset: Asset,
}

impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Check if the safety tax rate is less or equal to 1, if not raise an error
        decimal_param_le_one(&self.safety_tax_rate, "safety_tax_rate")?;
        Ok(())
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum ConfigError {
    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("Invalid Safety tax rate. Safety tax rate exceeds one")]
    InvalidSafetyTaxRate {},
}

