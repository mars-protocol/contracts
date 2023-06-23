use std::str::FromStr;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal;
use mars_params::types::{
    asset::{AssetParams, CmSettings, LiquidationBonus, RedBankSettings},
    hls::{HlsAssetType, HlsParams},
};

#[cw_serde]
pub struct CoinInfo {
    pub denom: String,
    pub price: Decimal,
    pub params: AssetParams,
}

pub fn umars_info() -> CoinInfo {
    let denom = "umars".to_string();
    CoinInfo {
        denom: denom.clone(),
        price: Decimal::from_atomics(1u128, 0).unwrap(),
        params: AssetParams {
            denom,
            max_loan_to_value: Decimal::from_atomics(8u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(84u128, 2).unwrap(),
            liquidation_bonus: LiquidationBonus {
                starting_lb: Decimal::percent(1u64),
                slope: Decimal::from_atomics(2u128, 0).unwrap(),
                min_lb: Decimal::percent(2u64),
                max_lb: Decimal::percent(10u64),
            },
            credit_manager: CmSettings {
                whitelisted: true,
                hls: None,
            },
            red_bank: RedBankSettings {
                deposit_enabled: true,
                borrow_enabled: true,
                deposit_cap: Default::default(),
            },
            protocol_liquidation_fee: Decimal::percent(2u64),
        },
    }
}

pub fn udai_info() -> CoinInfo {
    let denom = "udai".to_string();
    CoinInfo {
        denom,
        price: Decimal::from_atomics(313451u128, 6).unwrap(),
        params: AssetParams {
            denom: "udai".to_string(),
            max_loan_to_value: Decimal::from_atomics(85u128, 2).unwrap(),
            liquidation_threshold: Decimal::from_atomics(9u128, 1).unwrap(),
            liquidation_bonus: LiquidationBonus {
                starting_lb: Decimal::percent(1u64),
                slope: Decimal::from_atomics(2u128, 0).unwrap(),
                min_lb: Decimal::percent(2u64),
                max_lb: Decimal::percent(10u64),
            },
            credit_manager: CmSettings {
                whitelisted: true,
                hls: None,
            },
            red_bank: RedBankSettings {
                deposit_enabled: true,
                borrow_enabled: true,
                deposit_cap: Default::default(),
            },
            protocol_liquidation_fee: Decimal::percent(2u64),
        },
    }
}

pub fn uluna_info() -> CoinInfo {
    let denom = "uluna".to_string();
    CoinInfo {
        denom: denom.clone(),
        price: Decimal::from_atomics(100u128, 1).unwrap(),
        params: AssetParams {
            denom,
            max_loan_to_value: Decimal::from_atomics(7u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
            liquidation_bonus: LiquidationBonus {
                starting_lb: Decimal::percent(1u64),
                slope: Decimal::from_atomics(2u128, 0).unwrap(),
                min_lb: Decimal::percent(2u64),
                max_lb: Decimal::percent(10u64),
            },
            credit_manager: CmSettings {
                whitelisted: true,
                hls: None,
            },
            red_bank: RedBankSettings {
                deposit_enabled: true,
                borrow_enabled: true,
                deposit_cap: Default::default(),
            },
            protocol_liquidation_fee: Decimal::percent(2u64),
        },
    }
}

pub fn ustars_info() -> CoinInfo {
    let denom = "ustars".to_string();
    CoinInfo {
        denom: denom.clone(),
        price: Decimal::from_atomics(5265478965412365487125u128, 12).unwrap(),
        params: AssetParams {
            denom,
            max_loan_to_value: Decimal::from_atomics(6u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(7u128, 1).unwrap(),
            liquidation_bonus: LiquidationBonus {
                starting_lb: Decimal::percent(1u64),
                slope: Decimal::from_atomics(2u128, 0).unwrap(),
                min_lb: Decimal::percent(2u64),
                max_lb: Decimal::percent(10u64),
            },
            credit_manager: CmSettings {
                whitelisted: true,
                hls: Some(HlsParams {
                    max_loan_to_value: Decimal::from_str("0.75").unwrap(),
                    liquidation_threshold: Decimal::from_str("0.8").unwrap(),
                    correlations: vec![HlsAssetType::Coin {
                        denom: "stStars".to_string(),
                    }],
                }),
            },
            red_bank: RedBankSettings {
                deposit_enabled: true,
                borrow_enabled: true,
                deposit_cap: Default::default(),
            },
            protocol_liquidation_fee: Decimal::percent(2u64),
        },
    }
}

pub fn ujuno_info() -> CoinInfo {
    let denom = "ujuno".to_string();
    CoinInfo {
        denom: denom.clone(),
        price: Decimal::from_atomics(7012302005u128, 3).unwrap(),
        params: AssetParams {
            denom,
            max_loan_to_value: Decimal::from_atomics(8u128, 1).unwrap(),
            liquidation_threshold: Decimal::from_atomics(9u128, 1).unwrap(),
            liquidation_bonus: LiquidationBonus {
                starting_lb: Decimal::percent(1u64),
                slope: Decimal::from_atomics(2u128, 0).unwrap(),
                min_lb: Decimal::percent(2u64),
                max_lb: Decimal::percent(10u64),
            },
            credit_manager: CmSettings {
                whitelisted: true,
                hls: None,
            },
            red_bank: RedBankSettings {
                deposit_enabled: true,
                borrow_enabled: true,
                deposit_cap: Default::default(),
            },
            protocol_liquidation_fee: Decimal::percent(2u64),
        },
    }
}
