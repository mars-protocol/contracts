use std::str::FromStr;

use cosmwasm_std::Decimal;
use mars_params::types::{
    asset::{AssetParamsUnchecked, CmSettings, RedBankSettings},
    hls::HlsParamsUnchecked,
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
            deposit_cap: Default::default(),
        },
        max_loan_to_value: Decimal::from_str("0.4523").unwrap(),
        liquidation_threshold: Decimal::from_str("0.5").unwrap(),
        liquidation_bonus: Decimal::from_atomics(9u128, 2).unwrap(),
    }
}
