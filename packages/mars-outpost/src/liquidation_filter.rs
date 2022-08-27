use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;

/// Global configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    /// Contract owner
    pub owner: Addr,
    /// Address provider returns addresses for all protocol contracts
    pub address_provider: Addr,
}

/// Liquidate under-collateralized native loans. Coins used to repay must be sent in the
/// transaction this call is made.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Liquidate {
    /// Denom of the collateral asset, which liquidator gets from the borrower
    pub collateral_denom: String,
    /// Denom of the debt asset
    pub debt_denom: String,
    /// The address of the borrower getting liquidated
    pub user_address: String,
    /// Whether the liquidator gets liquidated collateral in maToken (true) or
    /// the underlying collateral asset (false)
    pub receive_ma_token: bool,
}

pub mod msg {
    use crate::liquidation_filter::Liquidate;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
    pub struct InstantiateMsg {
        /// Contract owner
        pub owner: String,
        /// Address provider returns addresses for all protocol contracts
        pub address_provider: String,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum ExecuteMsg {
        /// Set emission per second for an asset to holders of its maToken
        LiquidateMany {
            array: Vec<Liquidate>,
        },

        /// Update contract config (only callable by owner)
        UpdateConfig {
            owner: Option<String>,
            address_provider: Option<String>,
        },
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum QueryMsg {
        /// Query contract config
        Config {},
    }
}
