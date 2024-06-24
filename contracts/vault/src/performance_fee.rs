use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Int128, Int256, StdResult, Uint128};

use crate::error::ContractError;

/// The number of seconds in an hour. Used for calculating the performance fee which is applied hourly.
const ONE_HOUR_IN_SEC: u64 = 3600u64;

/// The maximum performance fee per 1h that can be set (equal to 0.0046287042457349%).
/// It is equivalent to 50% per year.
const MAX_PERFORMANCE_FEE_RATE: Decimal = Decimal::raw(46287042457349);

#[cw_serde]
#[derive(Default)]
pub struct PerformanceFeeConfig {
    /// The percentage of the performance fee that will be charged on the profits
    pub fee_rate: Decimal,

    /// The interval in seconds at which the performance fee can be withdrawn by the manager
    pub withdrawal_interval: u64,
}

impl PerformanceFeeConfig {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.fee_rate > MAX_PERFORMANCE_FEE_RATE {
            return Err(ContractError::InvalidPerformanceFee {
                expected: MAX_PERFORMANCE_FEE_RATE,
                actual: self.fee_rate,
            });
        }

        Ok(())
    }
}

#[cw_serde]
pub struct PerformanceFeeState {
    /// The timestamp (sec) of the last fee withdrawal
    pub last_withdrawal: u64,

    /// The total amount of base tokens in the vault account in Credit Manager
    pub base_tokens_amt: Uint128,

    /// The accumulated profit and loss since the last fee withdrawal
    pub accumulated_pnl: Int128,

    /// The total fees that have been accumulated since the last fee withdrawal
    pub accumulated_fee: Uint128,
}

impl Default for PerformanceFeeState {
    fn default() -> Self {
        Self {
            last_withdrawal: u64::MAX,
            base_tokens_amt: Uint128::zero(),
            accumulated_pnl: Int128::zero(),
            accumulated_fee: Uint128::zero(),
        }
    }
}

impl PerformanceFeeState {
    pub fn update_fee_and_pnl(
        &mut self,
        current_time: u64,
        total_base_tokens: Uint128,
        config: &PerformanceFeeConfig,
    ) -> StdResult<()> {
        // initial state, first time update by deposit
        if self.last_withdrawal == u64::MAX {
            self.last_withdrawal = current_time;
            return Ok(());
        }

        let accumulated_pnl_i256 = Int256::from(self.accumulated_pnl)
            + (Int256::from(total_base_tokens) - Int256::from(self.base_tokens_amt));
        // should be safe to convert to i128, the value should be in the range of i128
        let accumulated_pnl_i128: Int128 = accumulated_pnl_i256.try_into()?;

        // calculate the accumulated fee only if pnl is positive
        let accumulated_fee = if accumulated_pnl_i128 > Int128::zero() {
            let rate = self.calculate_time_based_performance_fee(current_time, config)?;
            accumulated_pnl_i128.unsigned_abs() * rate
        } else {
            Uint128::zero()
        };

        self.accumulated_pnl = accumulated_pnl_i128;
        self.accumulated_fee = accumulated_fee;

        Ok(())
    }

    fn calculate_time_based_performance_fee(
        &self,
        current_time: u64,
        config: &PerformanceFeeConfig,
    ) -> StdResult<Decimal> {
        let time_diff_in_sec = current_time - self.last_withdrawal;
        let time_diff_in_hours = time_diff_in_sec / ONE_HOUR_IN_SEC;
        Ok(config.fee_rate.checked_mul(Decimal::from_ratio(time_diff_in_hours, 1u128))?)
    }

    pub fn update_base_tokens_after_deposit(
        &mut self,
        total_base_tokens: Uint128,
        deposit_amt: Uint128,
    ) -> StdResult<()> {
        let updated_liquidity = total_base_tokens + deposit_amt;
        self.base_tokens_amt = updated_liquidity;
        Ok(())
    }

    pub fn update_base_tokens_after_redeem(
        &mut self,
        total_base_tokens: Uint128,
        withdraw_amt: Uint128,
    ) -> StdResult<()> {
        let updated_liquidity = total_base_tokens - withdraw_amt;
        self.base_tokens_amt = updated_liquidity;
        Ok(())
    }

    pub fn reset_state_by_manager(
        &mut self,
        current_time: u64,
        total_base_tokens: Uint128,
        config: &PerformanceFeeConfig,
    ) -> Result<(), ContractError> {
        if self.accumulated_fee.is_zero() {
            return Err(ContractError::ZeroPerformanceFee {});
        }

        let time_diff = current_time - self.last_withdrawal;
        if time_diff < config.withdrawal_interval {
            return Err(ContractError::WithdrawalIntervalNotPassed {});
        }

        let updated_liquidity = total_base_tokens - self.accumulated_fee;

        self.last_withdrawal = current_time;
        self.accumulated_pnl = Int128::zero();
        self.accumulated_fee = Uint128::zero();
        self.base_tokens_amt = updated_liquidity;

        Ok(())
    }
}
