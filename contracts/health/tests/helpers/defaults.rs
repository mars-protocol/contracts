use std::str::FromStr;

use cosmwasm_std::Decimal;
use mars_types::params::{
    AssetParamsUnchecked, CmSettings, HlsParamsUnchecked, LiquidationBonus, RedBankSettings,
};

pub fn default_asset_params(denom: &str) -> AssetParamsUnchecked {
    AssetParamsUnchecked {
        denom: denom.to_string(),
        credit_manager: CmSettings {
            whitelisted: true,
            hls: Some(HlsParamsUnchecked {
                max_loan_to_value: Decimal::from_str("0.8").unwrap(),
                liquidation_threshold: Decimal::from_str("0.9").unwrap(),
                correlations: vec![],
            }),
        },
        red_bank: RedBankSettings {
            deposit_enabled: false,
            borrow_enabled: false,
        },
        max_loan_to_value: Decimal::from_str("0.4523").unwrap(),
        liquidation_threshold: Decimal::from_str("0.5").unwrap(),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::percent(1u64),
            slope: Decimal::from_atomics(2u128, 0).unwrap(),
            min_lb: Decimal::percent(2u64),
            max_lb: Decimal::percent(10u64),
        },
        protocol_liquidation_fee: Decimal::percent(2u64),
        deposit_cap: Default::default(),
    }
}
