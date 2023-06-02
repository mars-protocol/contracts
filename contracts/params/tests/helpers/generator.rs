use std::str::FromStr;

use cosmwasm_std::{coin, Decimal, Uint128};
use mars_params::types::{
    AssetParams, HighLeverageStrategyParams, RedBankSettings, RoverSettings, VaultConfig,
};

pub fn default_asset_params() -> AssetParams {
    AssetParams {
        rover: RoverSettings {
            whitelisted: false,
            hls: HighLeverageStrategyParams {
                max_loan_to_value: Decimal::from_str("0.85").unwrap(),
                liquidation_threshold: Decimal::from_str("0.9").unwrap(),
            },
        },
        red_bank: RedBankSettings {
            deposit_enabled: true,
            borrow_enabled: false,
            deposit_cap: Uint128::new(1_000_000_000),
        },
        max_loan_to_value: Decimal::from_str("0.6").unwrap(),
        liquidation_threshold: Decimal::from_str("0.7").unwrap(),
        liquidation_bonus: Decimal::from_str("0.15").unwrap(),
    }
}

pub fn default_vault_config() -> VaultConfig {
    VaultConfig {
        deposit_cap: coin(100_000_000_000, "uusdc"),
        max_loan_to_value: Decimal::from_str("0.47").unwrap(),
        liquidation_threshold: Decimal::from_str("0.5").unwrap(),
        whitelisted: true,
    }
}
