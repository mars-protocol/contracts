use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use cosmwasm_std::{Env, StdResult};

use crate::math::decimal::Decimal;

use super::Market;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InterestRateModel {
    Dynamic {
        params: DynamicInterestRateModelParams,
        state: DynamicInterestRateModelState,
    },
    Linear {
        params: LinearInterestRateModelParams,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InterestRateModelParams {
    Dynamic(DynamicInterestRateModelParams),
    Linear(LinearInterestRateModelParams),
}

impl InterestRateModelParams {
    pub fn validate(&self) -> Result<(), InterestRateModelError> {
        match self {
            InterestRateModelParams::Dynamic(dynamic) => dynamic.validate(),
            InterestRateModelParams::Linear(linear) => linear.validate(),
        }
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum InterestRateModelError {
    #[error("max_borrow_rate should be greater than or equal to min_borrow_rate. max_borrow_rate: {max_borrow_rate:?}, min_borrow_rate: {min_borrow_rate:?}")]
    InvalidMinMaxBorrowRate {
        max_borrow_rate: Decimal,
        min_borrow_rate: Decimal,
    },

    #[error("Optimal utilization rate can't be greater than one")]
    InvalidOptimalUtilizationRate {},
}

pub fn init_interest_rate_model(
    params: InterestRateModelParams,
    current_block_time: u64,
) -> Result<InterestRateModel, InterestRateModelError> {
    params.validate()?;

    match params {
        InterestRateModelParams::Dynamic(dynamic_params) => {
            let state = DynamicInterestRateModelState {
                txs_since_last_borrow_rate_update: 0,
                borrow_rate_last_updated: current_block_time,
            };

            Ok(InterestRateModel::Dynamic {
                params: dynamic_params,
                state,
            })
        }
        InterestRateModelParams::Linear(linear_params) => Ok(InterestRateModel::Linear {
            params: linear_params,
        }),
    }
}

/// Updates market with new borrow/liquidity and interest rate model state
pub fn update_market_interest_rates_with_model(
    env: &Env,
    market: &mut Market,
    current_utilization_rate: Decimal,
) -> StdResult<()> {
    // update borrow rate
    match market.interest_rate_model {
        InterestRateModel::Dynamic {
            ref params,
            ref mut state,
        } => {
            let current_block_time = env.block.time.seconds();

            // update tx count and determine if borrow rate should be updated
            state.txs_since_last_borrow_rate_update += 1;
            let seconds_since_last_borrow_rate_update =
                current_block_time - state.borrow_rate_last_updated;

            let threshold_is_met = (state.txs_since_last_borrow_rate_update
                >= params.update_threshold_txs)
                || (seconds_since_last_borrow_rate_update >= params.update_threshold_seconds);

            // don't allow more than one update in the same block
            // this prevents calling the contract multiple times to set interest to min or max
            // on a single block.
            let should_update_borrow_rate =
                threshold_is_met && (seconds_since_last_borrow_rate_update != 0);

            if should_update_borrow_rate {
                market.borrow_rate =
                    dynamic_get_borrow_rate(params, current_utilization_rate, market.borrow_rate)?;
                state.txs_since_last_borrow_rate_update = 0;
                state.borrow_rate_last_updated = current_block_time;
            }
        }

        InterestRateModel::Linear { ref params } => {
            market.borrow_rate = linear_get_borrow_rate(params, current_utilization_rate)?;
        }
    }

    // update liquidity rate
    market.liquidity_rate = get_liquidity_rate(
        market.borrow_rate,
        current_utilization_rate,
        market.reserve_factor,
    )?;

    Ok(())
}

pub fn get_liquidity_rate(
    borrow_rate: Decimal,
    current_utilization_rate: Decimal,
    reserve_factor: Decimal,
) -> StdResult<Decimal> {
    borrow_rate
        .checked_mul(current_utilization_rate)?
        // This operation should not underflow as reserve_factor is checked to be <= 1
        .checked_mul(Decimal::one() - reserve_factor)
}

// DYNAMIC

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DynamicInterestRateModelParams {
    /// Minimum borrow rate
    pub min_borrow_rate: Decimal,
    /// Maximum borrow rate
    pub max_borrow_rate: Decimal,

    /// Optimal utilization rate targeted by the PID controller. Interest rate will decrease when lower and increase when higher
    pub optimal_utilization_rate: Decimal,

    /// Proportional parameter for the PID controller
    pub kp_1: Decimal,
    /// Kp value when error threshold is exceeded
    pub kp_2: Decimal,
    /// Min error that triggers Kp augmentation
    pub kp_augmentation_threshold: Decimal,

    /// Amount of transactions involving the market's interest update
    /// since last borrow rate update that will trigger
    /// the next borrow rate update
    pub update_threshold_txs: u32,
    /// Amount of seconds since last borrow rate update that will trigger
    /// the next borrow rate update when the next transaction involving the market's interest
    /// update happens
    pub update_threshold_seconds: u64,
}

impl DynamicInterestRateModelParams {
    pub fn validate(&self) -> Result<(), InterestRateModelError> {
        if self.min_borrow_rate > self.max_borrow_rate {
            return Err(InterestRateModelError::InvalidMinMaxBorrowRate {
                min_borrow_rate: self.min_borrow_rate,
                max_borrow_rate: self.max_borrow_rate,
            });
        }

        if self.optimal_utilization_rate > Decimal::one() {
            return Err(InterestRateModelError::InvalidOptimalUtilizationRate {});
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DynamicInterestRateModelState {
    pub txs_since_last_borrow_rate_update: u32,
    pub borrow_rate_last_updated: u64,
}

pub fn dynamic_get_borrow_rate(
    params: &DynamicInterestRateModelParams,
    current_utilization_rate: Decimal,
    current_borrow_rate: Decimal,
) -> StdResult<Decimal> {
    // error_value is unsigned
    // we use a boolean flag to determine the direction of the error
    let (error_value, error_positive) =
        if params.optimal_utilization_rate > current_utilization_rate {
            (
                params.optimal_utilization_rate - current_utilization_rate,
                true,
            )
        } else {
            (
                current_utilization_rate - params.optimal_utilization_rate,
                false,
            )
        };

    let kp = if error_value >= params.kp_augmentation_threshold {
        params.kp_2
    } else {
        params.kp_1
    };

    let p = kp.checked_mul(error_value)?;
    let mut new_borrow_rate = if error_positive {
        // error_positive = true (u_optimal > u) means we want utilization rate to go up
        // we lower interest rate so more people borrow
        if current_borrow_rate > p {
            current_borrow_rate - p
        } else {
            Decimal::zero()
        }
    } else {
        // error_positive = false (u_optimal < u) means we want utilization rate to go down
        // we increase interest rate so less people borrow
        current_borrow_rate + p
    };

    // Check borrow rate conditions
    if new_borrow_rate < params.min_borrow_rate {
        new_borrow_rate = params.min_borrow_rate
    } else if new_borrow_rate > params.max_borrow_rate {
        new_borrow_rate = params.max_borrow_rate;
    };

    Ok(new_borrow_rate)
}

// LINEAR

/// Linear interest rate model
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LinearInterestRateModelParams {
    /// Optimal utilization rate
    pub optimal_utilization_rate: Decimal,
    /// Base rate
    pub base: Decimal,
    /// Slope parameter for interest rate model function when utilization_rate < optimal_utilization_rate
    pub slope_1: Decimal,
    /// Slope parameter for interest rate model function when utilization_rate >= optimal_utilization_rate
    pub slope_2: Decimal,
}

impl LinearInterestRateModelParams {
    pub fn validate(&self) -> Result<(), InterestRateModelError> {
        if self.optimal_utilization_rate > Decimal::one() {
            return Err(InterestRateModelError::InvalidOptimalUtilizationRate {});
        }

        Ok(())
    }
}

pub fn linear_get_borrow_rate(
    params: &LinearInterestRateModelParams,
    current_utilization_rate: Decimal,
) -> StdResult<Decimal> {
    let new_borrow_rate = if current_utilization_rate <= params.optimal_utilization_rate {
        if current_utilization_rate.is_zero() {
            // prevent division by zero when optimal_utilization_rate is zero
            params.base
        } else {
            // The borrow interest rates increase slowly with utilization
            params.base
                + params.slope_1.checked_mul(
                    current_utilization_rate.checked_div(params.optimal_utilization_rate)?,
                )?
        }
    } else {
        // The borrow interest rates increase sharply with utilization
        params.base
            + params.slope_1
            + params
                .slope_2
                .checked_mul(current_utilization_rate - params.optimal_utilization_rate)?
                .checked_div(Decimal::one() - params.optimal_utilization_rate)?
    };

    Ok(new_borrow_rate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::decimal::Decimal;
    use crate::testing::mock_env_at_block_time;

    #[test]
    fn test_dynamic_model_lifecycle() {
        let optimal_utilization_rate = Decimal::percent(50);
        let kp_1 = Decimal::from_ratio(4u128, 100u128);
        let reserve_factor = Decimal::percent(10);

        let interest_rate_model_params = DynamicInterestRateModelParams {
            min_borrow_rate: Decimal::percent(0),
            max_borrow_rate: Decimal::percent(300),

            kp_1,
            optimal_utilization_rate,
            kp_augmentation_threshold: Decimal::percent(20),
            kp_2: Decimal::from_ratio(3u128, 1u128),

            update_threshold_txs: 2,
            update_threshold_seconds: 1000,
        };

        let time_start = 1_634_710_268;

        let interest_rate_model = init_interest_rate_model(
            InterestRateModelParams::Dynamic(interest_rate_model_params),
            time_start,
        )
        .unwrap();

        let mut market = Market {
            borrow_rate: Decimal::percent(10),
            liquidity_rate: Decimal::percent(10),
            reserve_factor: reserve_factor,
            interest_rate_model,
            ..Default::default()
        };

        // first update after 100 seconds borrow rate does not update
        {
            let diff = Decimal::percent(10);
            let utilization_rate = optimal_utilization_rate - diff;
            let previous_borrow_rate = market.borrow_rate;

            update_market_interest_rates_with_model(
                &mock_env_at_block_time(time_start + 100),
                &mut market,
                utilization_rate,
            )
            .unwrap();

            assert_eq!(market.borrow_rate, previous_borrow_rate);
            assert_eq!(
                market.liquidity_rate,
                previous_borrow_rate
                    .checked_mul(utilization_rate)
                    .unwrap()
                    .checked_mul(Decimal::one() - reserve_factor)
                    .unwrap()
            );
            if let InterestRateModel::Dynamic { ref state, .. } = market.interest_rate_model {
                assert_eq!(state.borrow_rate_last_updated, time_start);
                assert_eq!(state.txs_since_last_borrow_rate_update, 1);
            } else {
                panic!("Wrong interest rate model type");
            }
        }

        // second update after 200 seconds borrow rate updates (because tx threshold is reached)
        {
            let diff = Decimal::percent(5);
            let utilization_rate = optimal_utilization_rate - diff;
            let previous_borrow_rate = market.borrow_rate;

            update_market_interest_rates_with_model(
                &mock_env_at_block_time(time_start + 200),
                &mut market,
                utilization_rate,
            )
            .unwrap();

            let expected_borrow_rate = previous_borrow_rate - (kp_1.checked_mul(diff).unwrap());
            assert_eq!(market.borrow_rate, expected_borrow_rate);
            assert_eq!(
                market.liquidity_rate,
                expected_borrow_rate
                    .checked_mul(utilization_rate)
                    .unwrap()
                    .checked_mul(Decimal::one() - reserve_factor)
                    .unwrap()
            );
            if let InterestRateModel::Dynamic { ref state, .. } = market.interest_rate_model {
                assert_eq!(state.borrow_rate_last_updated, time_start + 200);
                assert_eq!(state.txs_since_last_borrow_rate_update, 0);
            } else {
                panic!("Wrong interest rate model type");
            }
        }

        // third update after 1201 seconds borrow rate updates (because seconds threshold is reached)
        {
            let diff = Decimal::percent(15);
            let utilization_rate = optimal_utilization_rate + diff;
            let previous_borrow_rate = market.borrow_rate;

            update_market_interest_rates_with_model(
                &mock_env_at_block_time(time_start + 1201),
                &mut market,
                utilization_rate,
            )
            .unwrap();

            let expected_borrow_rate = previous_borrow_rate + (kp_1.checked_mul(diff).unwrap());
            assert_eq!(market.borrow_rate, expected_borrow_rate);
            assert_eq!(
                market.liquidity_rate,
                expected_borrow_rate
                    .checked_mul(utilization_rate)
                    .unwrap()
                    .checked_mul(Decimal::one() - reserve_factor)
                    .unwrap()
            );
            if let InterestRateModel::Dynamic { ref state, .. } = market.interest_rate_model {
                assert_eq!(state.borrow_rate_last_updated, time_start + 1201);
                assert_eq!(state.txs_since_last_borrow_rate_update, 0);
            } else {
                panic!("Wrong interest rate model type");
            }
        }

        //  do three after 1201 seconds, borrow rate does not update (because even though txs are
        //  reached, it is on the same block)
        {
            let diff = Decimal::percent(10);
            let utilization_rate = optimal_utilization_rate + diff;
            let previous_borrow_rate = market.borrow_rate;

            update_market_interest_rates_with_model(
                &mock_env_at_block_time(time_start + 1201),
                &mut market,
                utilization_rate,
            )
            .unwrap();

            update_market_interest_rates_with_model(
                &mock_env_at_block_time(time_start + 1201),
                &mut market,
                utilization_rate,
            )
            .unwrap();

            update_market_interest_rates_with_model(
                &mock_env_at_block_time(time_start + 1201),
                &mut market,
                utilization_rate,
            )
            .unwrap();

            let expected_borrow_rate = previous_borrow_rate;
            assert_eq!(market.borrow_rate, expected_borrow_rate);
            assert_eq!(
                market.liquidity_rate,
                expected_borrow_rate
                    .checked_mul(utilization_rate)
                    .unwrap()
                    .checked_mul(Decimal::one() - reserve_factor)
                    .unwrap()
            );
            if let InterestRateModel::Dynamic { ref state, .. } = market.interest_rate_model {
                assert_eq!(state.borrow_rate_last_updated, time_start + 1201);
                assert_eq!(state.txs_since_last_borrow_rate_update, 3);
            } else {
                panic!("Wrong interest rate model type");
            }
        }

        // Updating after that works
        {
            let diff = Decimal::percent(15);
            let utilization_rate = optimal_utilization_rate + diff;
            let previous_borrow_rate = market.borrow_rate;

            update_market_interest_rates_with_model(
                &mock_env_at_block_time(time_start + 1208),
                &mut market,
                utilization_rate,
            )
            .unwrap();

            let expected_borrow_rate = previous_borrow_rate + (kp_1.checked_mul(diff).unwrap());
            assert_eq!(market.borrow_rate, expected_borrow_rate);
            assert_eq!(
                market.liquidity_rate,
                expected_borrow_rate
                    .checked_mul(utilization_rate)
                    .unwrap()
                    .checked_mul(Decimal::one() - reserve_factor)
                    .unwrap()
            );
            if let InterestRateModel::Dynamic { ref state, .. } = market.interest_rate_model {
                assert_eq!(state.borrow_rate_last_updated, time_start + 1208);
                assert_eq!(state.txs_since_last_borrow_rate_update, 0);
            } else {
                panic!("Wrong interest rate model type");
            }
        }
    }

    #[test]
    fn test_dynamic_borrow_rate_calculation() {
        let borrow_rate = Decimal::percent(5);
        let dynamic_ir_params = DynamicInterestRateModelParams {
            min_borrow_rate: Decimal::percent(1),
            max_borrow_rate: Decimal::percent(90),

            kp_1: Decimal::from_ratio(2u128, 1u128),
            optimal_utilization_rate: Decimal::percent(60),
            kp_augmentation_threshold: Decimal::percent(10),
            kp_2: Decimal::from_ratio(3u128, 1u128),

            update_threshold_txs: 1,
            update_threshold_seconds: 0,
        };

        // current utilization rate > optimal utilization rate
        {
            let current_utilization_rate = Decimal::percent(61);
            let new_borrow_rate =
                dynamic_get_borrow_rate(&dynamic_ir_params, current_utilization_rate, borrow_rate)
                    .unwrap();

            let expected_error =
                current_utilization_rate - dynamic_ir_params.optimal_utilization_rate;
            // we want to increase borrow rate to decrease utilization rate
            let expected_borrow_rate =
                borrow_rate + dynamic_ir_params.kp_1.checked_mul(expected_error).unwrap();

            assert_eq!(new_borrow_rate, expected_borrow_rate);
        }

        // current utilization rate < optimal utilization rate
        {
            let current_utilization_rate = Decimal::percent(59);
            let new_borrow_rate =
                dynamic_get_borrow_rate(&dynamic_ir_params, current_utilization_rate, borrow_rate)
                    .unwrap();

            let expected_error =
                dynamic_ir_params.optimal_utilization_rate - current_utilization_rate;
            // we want to decrease borrow rate to increase utilization rate
            let expected_borrow_rate =
                borrow_rate - Decimal::checked_mul(dynamic_ir_params.kp_1, expected_error).unwrap();

            assert_eq!(new_borrow_rate, expected_borrow_rate);
        }

        // current utilization rate > optimal utilization rate, increment KP by a multiplier if error goes beyond threshold
        {
            let current_utilization_rate = Decimal::percent(72);
            let new_borrow_rate =
                dynamic_get_borrow_rate(&dynamic_ir_params, current_utilization_rate, borrow_rate)
                    .unwrap();

            let expected_error =
                current_utilization_rate - dynamic_ir_params.optimal_utilization_rate;
            // we want to increase borrow rate to decrease utilization rate
            let expected_borrow_rate =
                borrow_rate + dynamic_ir_params.kp_2.checked_mul(expected_error).unwrap();

            assert_eq!(new_borrow_rate, expected_borrow_rate);
        }

        // current utilization rate < optimal utilization rate, borrow rate can't be less than min borrow rate
        {
            let current_utilization_rate = Decimal::percent(10);
            let new_borrow_rate =
                dynamic_get_borrow_rate(&dynamic_ir_params, current_utilization_rate, borrow_rate)
                    .unwrap();

            // we want to decrease borrow rate to increase utilization rate
            let expected_borrow_rate = dynamic_ir_params.min_borrow_rate;

            assert_eq!(new_borrow_rate, expected_borrow_rate);
        }

        // current utilization rate > optimal utilization rate, borrow rate can't be less than max borrow rate
        {
            let current_utilization_rate = Decimal::percent(90);
            let new_borrow_rate =
                dynamic_get_borrow_rate(&dynamic_ir_params, current_utilization_rate, borrow_rate)
                    .unwrap();

            // we want to increase borrow rate to decrease utilization rate
            let expected_borrow_rate = dynamic_ir_params.max_borrow_rate;

            assert_eq!(new_borrow_rate, expected_borrow_rate);
        }
    }

    #[test]
    fn test_linear_model_lifecycle() {
        let optimal_utilization_rate = Decimal::percent(80);
        let reserve_factor = Decimal::percent(20);

        let interest_rate_model_params = LinearInterestRateModelParams {
            optimal_utilization_rate,
            base: Decimal::from_ratio(0u128, 100u128),
            slope_1: Decimal::from_ratio(7u128, 100u128),
            slope_2: Decimal::from_ratio(45u128, 100u128),
        };

        let interest_rate_model = init_interest_rate_model(
            InterestRateModelParams::Linear(interest_rate_model_params.clone()),
            123,
        )
        .unwrap();

        let mut market = Market {
            borrow_rate: Decimal::percent(10),
            liquidity_rate: Decimal::zero(),
            reserve_factor: reserve_factor,
            interest_rate_model,
            ..Default::default()
        };

        let diff = Decimal::percent(10);
        let utilization_rate = optimal_utilization_rate - diff;

        update_market_interest_rates_with_model(
            &mock_env_at_block_time(1234),
            &mut market,
            utilization_rate,
        )
        .unwrap();

        let expected_borrow_rate = interest_rate_model_params.base
            + interest_rate_model_params
                .slope_1
                .checked_mul(utilization_rate)
                .unwrap()
                .checked_div(interest_rate_model_params.optimal_utilization_rate)
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
    fn test_linear_interest_rates_calculation() {
        let linear_ir_params = LinearInterestRateModelParams {
            optimal_utilization_rate: Decimal::percent(80),
            base: Decimal::from_ratio(0u128, 100u128),
            slope_1: Decimal::from_ratio(7u128, 100u128),
            slope_2: Decimal::from_ratio(45u128, 100u128),
        };

        // current utilization rate < optimal utilization rate
        {
            let current_utilization_rate = Decimal::percent(79);
            let new_borrow_rate =
                linear_get_borrow_rate(&linear_ir_params, current_utilization_rate).unwrap();

            let expected_borrow_rate = linear_ir_params.base
                + linear_ir_params
                    .slope_1
                    .checked_mul(current_utilization_rate)
                    .unwrap()
                    .checked_div(linear_ir_params.optimal_utilization_rate)
                    .unwrap();

            assert_eq!(new_borrow_rate, expected_borrow_rate);
        }

        // current utilization rate == optimal utilization rate
        {
            let current_utilization_rate = Decimal::percent(80);
            let new_borrow_rate =
                linear_get_borrow_rate(&linear_ir_params, current_utilization_rate).unwrap();

            let expected_borrow_rate = linear_ir_params.base
                + linear_ir_params
                    .slope_1
                    .checked_mul(current_utilization_rate)
                    .unwrap()
                    .checked_div(linear_ir_params.optimal_utilization_rate)
                    .unwrap();

            assert_eq!(new_borrow_rate, expected_borrow_rate);
        }

        // current utilization rate >= optimal utilization rate
        {
            let current_utilization_rate = Decimal::percent(81);
            let new_borrow_rate =
                linear_get_borrow_rate(&linear_ir_params, current_utilization_rate).unwrap();

            let expected_borrow_rate = linear_ir_params.base
                + linear_ir_params.slope_1
                + linear_ir_params
                    .slope_2
                    .checked_mul(
                        current_utilization_rate - linear_ir_params.optimal_utilization_rate,
                    )
                    .unwrap()
                    .checked_div(Decimal::one() - linear_ir_params.optimal_utilization_rate)
                    .unwrap();

            assert_eq!(new_borrow_rate, expected_borrow_rate);
        }

        // current utilization rate == 100% and optimal utilization rate == 100%
        {
            let linear_ir_params = LinearInterestRateModelParams {
                optimal_utilization_rate: Decimal::percent(100),
                base: Decimal::from_ratio(0u128, 100u128),
                slope_1: Decimal::from_ratio(7u128, 100u128),
                slope_2: Decimal::from_ratio(0u128, 100u128),
            };

            let current_utilization_rate = Decimal::percent(100);
            let new_borrow_rate =
                linear_get_borrow_rate(&linear_ir_params, current_utilization_rate).unwrap();

            let expected_borrow_rate = Decimal::percent(7);

            assert_eq!(new_borrow_rate, expected_borrow_rate);
        }

        // current utilization rate == 0% and optimal utilization rate == 0%
        {
            let linear_ir_params = LinearInterestRateModelParams {
                optimal_utilization_rate: Decimal::percent(0),
                base: Decimal::from_ratio(2u128, 100u128),
                slope_1: Decimal::from_ratio(7u128, 100u128),
                slope_2: Decimal::from_ratio(0u128, 100u128),
            };

            let current_utilization_rate = Decimal::percent(0);
            let new_borrow_rate =
                linear_get_borrow_rate(&linear_ir_params, current_utilization_rate).unwrap();

            let expected_borrow_rate = Decimal::percent(2);

            assert_eq!(new_borrow_rate, expected_borrow_rate);
        }

        // current utilization rate == 20% and optimal utilization rate == 0%
        {
            let linear_ir_params = LinearInterestRateModelParams {
                optimal_utilization_rate: Decimal::percent(0),
                base: Decimal::from_ratio(2u128, 100u128),
                slope_1: Decimal::from_ratio(1u128, 100u128),
                slope_2: Decimal::from_ratio(5u128, 100u128),
            };

            let current_utilization_rate = Decimal::percent(20);
            let new_borrow_rate =
                linear_get_borrow_rate(&linear_ir_params, current_utilization_rate).unwrap();

            let expected_borrow_rate = linear_ir_params.base
                + linear_ir_params.slope_1
                + linear_ir_params
                    .slope_2
                    .checked_mul(current_utilization_rate)
                    .unwrap();

            assert_eq!(new_borrow_rate, expected_borrow_rate);
        }
    }
}
