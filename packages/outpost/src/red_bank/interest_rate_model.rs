use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, StdError, StdResult};

use crate::{error::MarsError, helpers::decimal_param_le_one, math};

#[cw_serde]
#[derive(Eq, Default)]
pub struct InterestRateModel {
    /// Optimal utilization rate
    pub optimal_utilization_rate: Decimal,
    /// Base rate
    pub base: Decimal,
    /// Slope parameter for interest rate model function when utilization_rate < optimal_utilization_rate
    pub slope_1: Decimal,
    /// Slope parameter for interest rate model function when utilization_rate >= optimal_utilization_rate
    pub slope_2: Decimal,
}

impl InterestRateModel {
    pub fn validate(&self) -> Result<(), MarsError> {
        decimal_param_le_one(self.optimal_utilization_rate, "optimal_utilization_rate")?;
        Ok(())
    }

    pub fn get_borrow_rate(&self, current_utilization_rate: Decimal) -> StdResult<Decimal> {
        let new_borrow_rate = if current_utilization_rate <= self.optimal_utilization_rate {
            if current_utilization_rate.is_zero() {
                // prevent division by zero when optimal_utilization_rate is zero
                self.base
            } else {
                // The borrow interest rates increase slowly with utilization
                self.base
                    + self.slope_1.checked_mul(math::divide_decimal_by_decimal(
                        current_utilization_rate,
                        self.optimal_utilization_rate,
                    )?)?
            }
        } else {
            // The borrow interest rates increase sharply with utilization
            self.base
                + self.slope_1
                + math::divide_decimal_by_decimal(
                    self.slope_2
                        .checked_mul(current_utilization_rate - self.optimal_utilization_rate)?,
                    Decimal::one() - self.optimal_utilization_rate,
                )?
        };
        Ok(new_borrow_rate)
    }

    pub fn get_liquidity_rate(
        &self,
        borrow_rate: Decimal,
        current_utilization_rate: Decimal,
        reserve_factor: Decimal,
    ) -> StdResult<Decimal> {
        borrow_rate
            .checked_mul(current_utilization_rate)?
            // This operation should not underflow as reserve_factor is checked to be <= 1
            .checked_mul(Decimal::one() - reserve_factor)
            .map_err(StdError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::red_bank::Market;

    #[test]
    fn test_model_lifecycle() {
        let optimal_utilization_rate = Decimal::percent(80);
        let reserve_factor = Decimal::percent(20);

        let model = InterestRateModel {
            optimal_utilization_rate,
            base: Decimal::zero(),
            slope_1: Decimal::percent(7),
            slope_2: Decimal::percent(45),
        };

        let mut market = Market {
            borrow_rate: Decimal::percent(10),
            liquidity_rate: Decimal::zero(),
            reserve_factor,
            interest_rate_model: model.clone(),
            ..Default::default()
        };

        let diff = Decimal::percent(10);
        let utilization_rate = optimal_utilization_rate - diff;

        market.update_interest_rates(utilization_rate).unwrap();

        let expected_borrow_rate = model.base
            + math::divide_decimal_by_decimal(
                model.slope_1.checked_mul(utilization_rate).unwrap(),
                model.optimal_utilization_rate,
            )
            .unwrap();

        assert_eq!(market.borrow_rate, expected_borrow_rate);
        assert_eq!(
            market.liquidity_rate,
            expected_borrow_rate
                .checked_mul(utilization_rate)
                .unwrap()
                .checked_mul(Decimal::one() - reserve_factor)
                .unwrap()
        );
    }

    #[test]
    fn test_interest_rates_calculation() {
        let model = InterestRateModel {
            optimal_utilization_rate: Decimal::percent(80),
            base: Decimal::zero(),
            slope_1: Decimal::percent(7),
            slope_2: Decimal::percent(45),
        };

        // current utilization rate < optimal utilization rate
        {
            let current_utilization_rate = Decimal::percent(79);
            let new_borrow_rate = model.get_borrow_rate(current_utilization_rate).unwrap();

            let expected_borrow_rate = model.base
                + math::divide_decimal_by_decimal(
                    model.slope_1.checked_mul(current_utilization_rate).unwrap(),
                    model.optimal_utilization_rate,
                )
                .unwrap();

            assert_eq!(new_borrow_rate, expected_borrow_rate);
        }

        // current utilization rate == optimal utilization rate
        {
            let current_utilization_rate = Decimal::percent(80);
            let new_borrow_rate = model.get_borrow_rate(current_utilization_rate).unwrap();

            let expected_borrow_rate = model.base
                + math::divide_decimal_by_decimal(
                    model.slope_1.checked_mul(current_utilization_rate).unwrap(),
                    model.optimal_utilization_rate,
                )
                .unwrap();

            assert_eq!(new_borrow_rate, expected_borrow_rate);
        }

        // current utilization rate >= optimal utilization rate
        {
            let current_utilization_rate = Decimal::percent(81);
            let new_borrow_rate = model.get_borrow_rate(current_utilization_rate).unwrap();

            let expected_borrow_rate = model.base
                + model.slope_1
                + math::divide_decimal_by_decimal(
                    model
                        .slope_2
                        .checked_mul(current_utilization_rate - model.optimal_utilization_rate)
                        .unwrap(),
                    Decimal::one() - model.optimal_utilization_rate,
                )
                .unwrap();

            assert_eq!(new_borrow_rate, expected_borrow_rate);
        }

        // current utilization rate == 100% and optimal utilization rate == 100%
        {
            let model = InterestRateModel {
                optimal_utilization_rate: Decimal::percent(100),
                base: Decimal::zero(),
                slope_1: Decimal::percent(7),
                slope_2: Decimal::zero(),
            };

            let current_utilization_rate = Decimal::percent(100);
            let new_borrow_rate = model.get_borrow_rate(current_utilization_rate).unwrap();

            let expected_borrow_rate = Decimal::percent(7);

            assert_eq!(new_borrow_rate, expected_borrow_rate);
        }

        // current utilization rate == 0% and optimal utilization rate == 0%
        {
            let model = InterestRateModel {
                optimal_utilization_rate: Decimal::percent(0),
                base: Decimal::percent(2),
                slope_1: Decimal::percent(7),
                slope_2: Decimal::zero(),
            };

            let current_utilization_rate = Decimal::percent(0);
            let new_borrow_rate = model.get_borrow_rate(current_utilization_rate).unwrap();

            let expected_borrow_rate = Decimal::percent(2);

            assert_eq!(new_borrow_rate, expected_borrow_rate);
        }

        // current utilization rate == 20% and optimal utilization rate == 0%
        {
            let model = InterestRateModel {
                optimal_utilization_rate: Decimal::percent(0),
                base: Decimal::percent(2),
                slope_1: Decimal::percent(1),
                slope_2: Decimal::percent(5),
            };

            let current_utilization_rate = Decimal::percent(20);
            let new_borrow_rate = model.get_borrow_rate(current_utilization_rate).unwrap();

            let expected_borrow_rate = model.base
                + model.slope_1
                + model.slope_2.checked_mul(current_utilization_rate).unwrap();

            assert_eq!(new_borrow_rate, expected_borrow_rate);
        }
    }
}
