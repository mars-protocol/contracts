use std::str::FromStr;

use cosmwasm_std::{coin, Decimal, Uint128};
use mars_params::types::{
    AssetParams, AssetPermissions, RedBankPermissions, RoverPermissions, VaultConfig,
};
use mars_red_bank_types::red_bank::InterestRateModel;

pub fn default_asset_params() -> AssetParams {
    AssetParams {
        permissions: AssetPermissions {
            rover: RoverPermissions {
                whitelisted: false,
            },
            red_bank: RedBankPermissions {
                deposit_enabled: true,
                borrow_enabled: false,
            },
        },
        max_loan_to_value: Decimal::from_str("0.6").unwrap(),
        liquidation_threshold: Decimal::from_str("0.7").unwrap(),
        liquidation_bonus: Decimal::from_str("0.15").unwrap(),
        red_bank_deposit_cap: Uint128::new(1_000_000_000),
        interest_rate_model: InterestRateModel {
            optimal_utilization_rate: Decimal::from_str("0.6").unwrap(),
            base: Decimal::zero(),
            slope_1: Decimal::from_str("0.15").unwrap(),
            slope_2: Decimal::from_str("3").unwrap(),
        },
        reserve_factor: Decimal::from_str("0.2").unwrap(),
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
