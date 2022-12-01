use std::ops::Sub;

use cosmwasm_std::Decimal;

use mars_rover::msg::query::HealthResponse;

pub const MAX_VALUE_FOR_BURN: u128 = 1000u128;

pub fn generate_health_response(debt_value: u128, collateral_value: u128) -> HealthResponse {
    HealthResponse {
        total_debt_value: Decimal::from_atomics(debt_value, 0).unwrap(),
        total_collateral_value: Decimal::from_atomics(collateral_value, 0).unwrap(),
        max_ltv_adjusted_collateral: Default::default(),
        liquidation_threshold_adjusted_collateral: Default::default(),
        max_ltv_health_factor: None,
        liquidation_health_factor: None,
        liquidatable: false,
        above_max_ltv: false,
    }
}

pub fn below_max_for_burn() -> HealthResponse {
    HealthResponse {
        total_debt_value: Decimal::from_atomics(MAX_VALUE_FOR_BURN.sub(1), 0).unwrap(),
        total_collateral_value: Default::default(),
        max_ltv_adjusted_collateral: Default::default(),
        liquidation_threshold_adjusted_collateral: Default::default(),
        max_ltv_health_factor: None,
        liquidation_health_factor: None,
        liquidatable: false,
        above_max_ltv: false,
    }
}
