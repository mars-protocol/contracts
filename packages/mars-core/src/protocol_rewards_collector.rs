use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use cosmwasm_std::{Addr, Decimal as StdDecimal};

use crate::error::MarsError;
use crate::helpers::all_conditions_valid;
use crate::math::decimal::Decimal;

/// Global configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Contract owner
    pub owner: Addr,
    /// Address provider returns addresses for all protocol contracts
    pub address_provider_address: Addr,
    /// Percentage of fees that are sent to the safety fund
    pub safety_fund_fee_share: Decimal,
    /// Percentage of fees that are sent to the treasury
    pub treasury_fee_share: Decimal,
    /// Astroport factory contract address
    pub astroport_factory_address: Addr,
    /// Astroport max spread
    pub astroport_max_spread: StdDecimal,
}

impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        let conditions_and_names = vec![
            (
                Self::less_or_equal_one(&self.safety_fund_fee_share),
                "safety_fund_fee_share",
            ),
            (
                Self::less_or_equal_one(&self.treasury_fee_share),
                "treasury_fee_share",
            ),
        ];
        all_conditions_valid(conditions_and_names)?;

        let combined_fee_share = self.safety_fund_fee_share + self.treasury_fee_share;
        // Combined fee shares cannot exceed one
        if combined_fee_share > Decimal::one() {
            return Err(ConfigError::InvalidFeeShareAmounts {});
        }

        Ok(())
    }

    fn less_or_equal_one(value: &Decimal) -> bool {
        value.le(&Decimal::one())
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum ConfigError {
    #[error("{0}")]
    Mars(#[from] MarsError),

    #[error("Invalid fee share amounts. Sum of safety and treasury fee shares exceeds one")]
    InvalidFeeShareAmounts {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AssetConfig {
    pub enabled_for_distribution: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for AssetConfig {
    fn default() -> Self {
        AssetConfig {
            enabled_for_distribution: false,
        }
    }
}

pub mod msg {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    use cosmwasm_std::{CosmosMsg, Decimal as StdDecimal, Uint128};

    use astroport::asset::AssetInfo;

    use crate::asset::Asset;
    use crate::math::decimal::Decimal;

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    pub struct InstantiateMsg {
        pub config: CreateOrUpdateConfig,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    pub struct CreateOrUpdateConfig {
        pub owner: Option<String>,
        pub address_provider_address: Option<String>,
        pub safety_fund_fee_share: Option<Decimal>,
        pub treasury_fee_share: Option<Decimal>,
        pub astroport_factory_address: Option<String>,
        pub astroport_max_spread: Option<StdDecimal>,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum ExecuteMsg {
        /// Update contract config
        UpdateConfig { config: CreateOrUpdateConfig },

        /// Update asset config
        UpdateAssetConfig { asset: Asset, enabled: bool },

        /// Withdraw maTokens from the red bank
        WithdrawFromRedBank {
            asset: Asset,
            amount: Option<Uint128>,
        },

        /// Distribute the accrued protocol income to the safety fund, treasury and staking contracts,
        /// according to the split set in config.
        /// Callable by any address.
        DistributeProtocolRewards {
            /// Asset market fees to distribute
            asset: Asset,
            /// Amount to distribute to protocol contracts, defaults to contract balance if not specified
            amount: Option<Uint128>,
        },

        /// Swap any asset on the contract to uusd
        SwapAssetToUusd {
            offer_asset_info: AssetInfo,
            amount: Option<Uint128>,
        },

        /// Execute Cosmos msg (only callable by owner)
        ExecuteCosmosMsg(CosmosMsg),
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum QueryMsg {
        /// Get config parameters
        Config {},
        /// Get asset config parameters
        AssetConfig { asset: Asset },
    }
}
