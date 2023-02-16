use std::fmt;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};

#[cw_serde]
pub struct Health {
    /// The sum of the value of all debts
    pub total_debt_value: Uint128,
    /// The sum of the value of all collaterals
    pub total_collateral_value: Uint128,
    /// The sum of the value of all colletarals adjusted by their Max LTV
    pub max_ltv_adjusted_collateral: Uint128,
    /// The sum of the value of all colletarals adjusted by their Liquidation Threshold
    pub liquidation_threshold_adjusted_collateral: Uint128,
    /// The sum of the value of all collaterals multiplied by their max LTV, over the total value of debt
    pub max_ltv_health_factor: Option<Decimal>,
    /// The sum of the value of all collaterals multiplied by their liquidation threshold over the total value of debt
    pub liquidation_health_factor: Option<Decimal>,
}

impl fmt::Display for Health {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "(total_debt_value: {}, total_collateral_value: {},  max_ltv_adjusted_collateral: {}, lqdt_threshold_adjusted_collateral: {}, max_ltv_health_factor: {}, liquidation_health_factor: {})",
            self.total_debt_value,
            self.total_collateral_value,
            self.max_ltv_adjusted_collateral,
            self.liquidation_threshold_adjusted_collateral,
            self.max_ltv_health_factor.map_or("n/a".to_string(), |x| x.to_string()),
            self.liquidation_health_factor.map_or("n/a".to_string(), |x| x.to_string())
        )
    }
}

impl Health {
    #[inline]
    pub fn is_liquidatable(&self) -> bool {
        is_below_one(&self.liquidation_health_factor)
    }

    #[inline]
    pub fn is_above_max_ltv(&self) -> bool {
        is_below_one(&self.max_ltv_health_factor)
    }
}

pub fn is_below_one(health_factor: &Option<Decimal>) -> bool {
    health_factor.map_or(false, |hf| hf < Decimal::one())
}

#[cw_serde]
pub struct HealthResponse {
    pub total_debt_value: Uint128,
    pub total_collateral_value: Uint128,
    pub max_ltv_adjusted_collateral: Uint128,
    pub liquidation_threshold_adjusted_collateral: Uint128,
    pub max_ltv_health_factor: Option<Decimal>,
    pub liquidation_health_factor: Option<Decimal>,
    pub liquidatable: bool,
    pub above_max_ltv: bool,
}

impl From<Health> for HealthResponse {
    fn from(h: Health) -> Self {
        Self {
            total_debt_value: h.total_debt_value,
            total_collateral_value: h.total_collateral_value,
            max_ltv_adjusted_collateral: h.max_ltv_adjusted_collateral,
            liquidation_threshold_adjusted_collateral: h.liquidation_threshold_adjusted_collateral,
            max_ltv_health_factor: h.max_ltv_health_factor,
            liquidation_health_factor: h.liquidation_health_factor,
            liquidatable: h.is_liquidatable(),
            above_max_ltv: h.is_above_max_ltv(),
        }
    }
}
