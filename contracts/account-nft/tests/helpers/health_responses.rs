use std::ops::Sub;

use cosmwasm_std::Uint128;
use mars_rover_health_types::HealthValuesResponse;

pub const MAX_VALUE_FOR_BURN: Uint128 = Uint128::new(1000);

pub fn generate_health_response(debt_value: u128, collateral_value: u128) -> HealthValuesResponse {
    HealthValuesResponse {
        total_debt_value: debt_value.into(),
        total_collateral_value: collateral_value.into(),
        max_ltv_adjusted_collateral: Default::default(),
        liquidation_threshold_adjusted_collateral: Default::default(),
        max_ltv_health_factor: None,
        liquidation_health_factor: None,
        liquidatable: false,
        above_max_ltv: false,
    }
}

pub fn below_max_for_burn() -> HealthValuesResponse {
    HealthValuesResponse {
        total_debt_value: Default::default(),
        total_collateral_value: MAX_VALUE_FOR_BURN.sub(Uint128::one()),
        max_ltv_adjusted_collateral: Default::default(),
        liquidation_threshold_adjusted_collateral: Default::default(),
        max_ltv_health_factor: None,
        liquidation_health_factor: None,
        liquidatable: false,
        above_max_ltv: false,
    }
}
