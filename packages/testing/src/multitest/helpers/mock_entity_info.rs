use std::str::FromStr;

use cosmwasm_std::{coin, Decimal};
use cw_utils::Duration;
use mars_types::params::{HlsAssetType, HlsParamsUnchecked, LiquidationBonus};

use super::{CoinInfo, VaultTestInfo};

pub const ASTRO_LP_DENOM: &str =
    "factory/neutron1sf456kx85dz0wfjs4sx0s80dyzmc360pfc0rdzactxt8xrse9ykqsdpy2y/astroport/share";

pub fn coin_info(denom: &str) -> CoinInfo {
    CoinInfo {
        denom: denom.to_string(),
        price: Decimal::from_atomics(25u128, 2).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::percent(1u64),
            slope: Decimal::from_atomics(2u128, 0).unwrap(),
            min_lb: Decimal::percent(2u64),
            max_lb: Decimal::percent(10u64),
        },
        protocol_liquidation_fee: Decimal::percent(2u64),
        whitelisted: true,
        hls: None,
    }
}

pub fn uosmo_info() -> CoinInfo {
    CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(25u128, 2).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::percent(1u64),
            slope: Decimal::from_atomics(2u128, 0).unwrap(),
            min_lb: Decimal::percent(2u64),
            max_lb: Decimal::percent(10u64),
        },
        protocol_liquidation_fee: Decimal::percent(2u64),
        whitelisted: true,
        hls: None,
    }
}

pub fn uatom_info() -> CoinInfo {
    CoinInfo {
        denom: "uatom".to_string(),
        price: Decimal::from_atomics(10u128, 1).unwrap(),
        max_ltv: Decimal::from_atomics(82u128, 2).unwrap(),
        liquidation_threshold: Decimal::from_atomics(9u128, 1).unwrap(),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::percent(1u64),
            slope: Decimal::from_atomics(2u128, 0).unwrap(),
            min_lb: Decimal::percent(2u64),
            max_lb: Decimal::percent(10u64),
        },
        protocol_liquidation_fee: Decimal::percent(2u64),
        whitelisted: true,
        hls: Some(HlsParamsUnchecked {
            max_loan_to_value: Decimal::from_str("0.86").unwrap(),
            liquidation_threshold: Decimal::from_str("0.93").unwrap(),
            correlations: vec![
                HlsAssetType::Coin {
                    denom: "uatom".to_string(),
                },
                HlsAssetType::Coin {
                    denom: "stAtom".to_string(),
                },
                HlsAssetType::Coin {
                    denom: lp_token_info().denom,
                },
            ],
        }),
    }
}

pub fn ujake_info() -> CoinInfo {
    CoinInfo {
        denom: "ujake".to_string(),
        price: Decimal::from_atomics(23654u128, 4).unwrap(),
        max_ltv: Decimal::from_atomics(5u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap(),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::percent(1u64),
            slope: Decimal::from_atomics(2u128, 0).unwrap(),
            min_lb: Decimal::percent(2u64),
            max_lb: Decimal::percent(10u64),
        },
        protocol_liquidation_fee: Decimal::percent(2u64),
        whitelisted: true,
        hls: Some(HlsParamsUnchecked {
            max_loan_to_value: Decimal::from_str("0.7").unwrap(),
            liquidation_threshold: Decimal::from_str("0.8").unwrap(),
            correlations: vec![],
        }),
    }
}

pub fn blacklisted_coin_info() -> CoinInfo {
    CoinInfo {
        denom: "uluna".to_string(),
        price: Decimal::from_str("0.01").unwrap(),
        max_ltv: Decimal::from_str("0.4").unwrap(),
        liquidation_threshold: Decimal::from_str("0.5").unwrap(),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::percent(1u64),
            slope: Decimal::from_atomics(2u128, 0).unwrap(),
            min_lb: Decimal::percent(2u64),
            max_lb: Decimal::percent(10u64),
        },
        protocol_liquidation_fee: Decimal::percent(2u64),
        whitelisted: false,
        hls: None,
    }
}

pub fn lp_token_info() -> CoinInfo {
    CoinInfo {
        denom: "ugamm22".to_string(),
        price: Decimal::from_atomics(9874u128, 3).unwrap(),
        max_ltv: Decimal::from_atomics(63u128, 2).unwrap(),
        liquidation_threshold: Decimal::from_atomics(68u128, 2).unwrap(),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::percent(1u64),
            slope: Decimal::from_atomics(2u128, 0).unwrap(),
            min_lb: Decimal::percent(2u64),
            max_lb: Decimal::percent(10u64),
        },
        protocol_liquidation_fee: Decimal::percent(40u64),
        whitelisted: true,
        hls: Some(HlsParamsUnchecked {
            max_loan_to_value: Decimal::from_str("0.75").unwrap(),
            liquidation_threshold: Decimal::from_str("0.82").unwrap(),
            correlations: vec![],
        }),
    }
}

pub fn locked_vault_info() -> VaultTestInfo {
    generate_mock_vault(Some(Duration::Time(1_209_600))) // 14 days)
}

pub fn unlocked_vault_info() -> VaultTestInfo {
    generate_mock_vault(None)
}

pub fn generate_mock_vault(lockup: Option<Duration>) -> VaultTestInfo {
    let vault_token_denom = if lockup.is_some() {
        "uleverage-locked".to_string()
    } else {
        "uleverage-unlocked".to_string()
    };

    let lp_token = lp_token_info();
    VaultTestInfo {
        vault_token_denom,
        lockup,
        base_token_denom: lp_token.denom.clone(),
        deposit_cap: coin(10_000_000, "uusdc"),
        max_ltv: Decimal::from_str("0.6").unwrap(),
        liquidation_threshold: Decimal::from_str("0.7").unwrap(),
        whitelisted: true,
        hls: Some(HlsParamsUnchecked {
            max_loan_to_value: lp_token.hls.as_ref().unwrap().max_loan_to_value,
            liquidation_threshold: lp_token.hls.unwrap().liquidation_threshold,
            correlations: vec![],
        }),
    }
}
