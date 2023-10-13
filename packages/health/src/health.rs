use std::{collections::HashMap, fmt};

use cosmwasm_std::{Addr, Coin, Decimal, Fraction, QuerierWrapper, StdResult, Uint128};
use mars_types::{health::HealthValuesResponse, params::AssetParams};

use crate::{error::HealthError, query::MarsQuerier};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Position {
    pub denom: String,
    pub price: Decimal,
    pub collateral_amount: Uint128,
    pub debt_amount: Uint128,
    pub max_ltv: Decimal,
    pub liquidation_threshold: Decimal,
}

#[derive(Default, Debug, PartialEq, Eq)]
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

impl From<HealthValuesResponse> for Health {
    fn from(h: HealthValuesResponse) -> Self {
        Self {
            total_debt_value: h.total_debt_value,
            total_collateral_value: h.total_collateral_value,
            max_ltv_adjusted_collateral: h.max_ltv_adjusted_collateral,
            liquidation_threshold_adjusted_collateral: h.liquidation_threshold_adjusted_collateral,
            max_ltv_health_factor: h.max_ltv_health_factor,
            liquidation_health_factor: h.liquidation_health_factor,
        }
    }
}

impl Health {
    /// Compute the health from coins (collateral and debt)
    pub fn compute_health_from_coins(
        querier: &QuerierWrapper,
        oracle_addr: &Addr,
        red_bank_addr: &Addr,
        collateral: &[Coin],
        debt: &[Coin],
    ) -> Result<Health, HealthError> {
        let querier = MarsQuerier::new(querier, oracle_addr, red_bank_addr);
        let positions = Self::positions_from_coins(&querier, collateral, debt)?;

        Self::compute_health(&positions.into_values().collect::<Vec<_>>())
    }

    /// Compute the health for a Position
    pub fn compute_health(positions: &[Position]) -> Result<Health, HealthError> {
        let mut health = positions.iter().try_fold::<_, _, Result<Health, HealthError>>(
            Health::default(),
            |mut h, p| {
                let collateral_value = p
                    .collateral_amount
                    .checked_multiply_ratio(p.price.numerator(), p.price.denominator())?;
                h.total_debt_value += p
                    .debt_amount
                    .checked_multiply_ratio(p.price.numerator(), p.price.denominator())?;
                h.total_collateral_value += collateral_value;
                h.max_ltv_adjusted_collateral += collateral_value
                    .checked_multiply_ratio(p.max_ltv.numerator(), p.max_ltv.denominator())?;
                h.liquidation_threshold_adjusted_collateral += collateral_value
                    .checked_multiply_ratio(
                        p.liquidation_threshold.numerator(),
                        p.liquidation_threshold.denominator(),
                    )?;
                Ok(h)
            },
        )?;

        // If there aren't any debts a health factor can't be computed (divide by zero)
        if !health.total_debt_value.is_zero() {
            health.max_ltv_health_factor = Some(Decimal::checked_from_ratio(
                health.max_ltv_adjusted_collateral,
                health.total_debt_value,
            )?);
            health.liquidation_health_factor = Some(Decimal::checked_from_ratio(
                health.liquidation_threshold_adjusted_collateral,
                health.total_debt_value,
            )?);
        }

        Ok(health)
    }

    #[inline]
    pub fn is_liquidatable(&self) -> bool {
        self.liquidation_health_factor.map_or(false, |hf| hf < Decimal::one())
    }

    #[inline]
    pub fn is_above_max_ltv(&self) -> bool {
        self.max_ltv_health_factor.map_or(false, |hf| hf < Decimal::one())
    }

    /// Convert a collection of coins (Collateral and debts) to a map of `Position`
    pub fn positions_from_coins(
        querier: &MarsQuerier,
        collateral: &[Coin],
        debt: &[Coin],
    ) -> StdResult<HashMap<String, Position>> {
        let mut positions: HashMap<String, Position> = HashMap::new();

        collateral.iter().try_for_each(|c| -> StdResult<_> {
            match positions.get_mut(&c.denom) {
                Some(p) => {
                    p.collateral_amount += c.amount;
                }
                None => {
                    let AssetParams {
                        max_loan_to_value,
                        liquidation_threshold,
                        ..
                    } = querier.query_asset_params(&c.denom)?;

                    positions.insert(
                        c.denom.clone(),
                        Position {
                            denom: c.denom.clone(),
                            collateral_amount: c.amount,
                            debt_amount: Uint128::zero(),
                            price: querier.query_price(&c.denom)?,
                            max_ltv: max_loan_to_value,
                            liquidation_threshold,
                        },
                    );
                }
            }
            Ok(())
        })?;

        debt.iter().try_for_each(|d| -> StdResult<_> {
            match positions.get_mut(&d.denom) {
                Some(p) => {
                    p.debt_amount += d.amount;
                }
                None => {
                    let AssetParams {
                        max_loan_to_value,
                        liquidation_threshold,
                        ..
                    } = querier.query_asset_params(&d.denom)?;

                    positions.insert(
                        d.denom.clone(),
                        Position {
                            denom: d.denom.clone(),
                            collateral_amount: Uint128::zero(),
                            debt_amount: d.amount,
                            price: querier.query_price(&d.denom)?,
                            max_ltv: max_loan_to_value,
                            liquidation_threshold,
                        },
                    );
                }
            }
            Ok(())
        })?;
        Ok(positions)
    }
}
