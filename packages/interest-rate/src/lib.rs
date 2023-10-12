use cosmwasm_std::{Decimal, StdError, StdResult, Uint128};
use mars_types::{error::MarsError, red_bank::Market};

/// Scaling factor used to keep more precision during division / multiplication by index.
pub const SCALING_FACTOR: Uint128 = Uint128::new(1_000_000);

const SECONDS_PER_YEAR: u64 = 31536000u64;

pub fn calculate_applied_linear_interest_rate(
    index: Decimal,
    rate: Decimal,
    time_elapsed: u64,
) -> StdResult<Decimal> {
    let rate_factor = rate.checked_mul(Decimal::from_ratio(
        Uint128::from(time_elapsed),
        Uint128::from(SECONDS_PER_YEAR),
    ))?;
    index.checked_mul(Decimal::one() + rate_factor).map_err(StdError::from)
}

/// Get scaled liquidity amount from an underlying amount, a Market and timestamp in seconds
/// Liquidity amounts are always truncated to make sure rounding errors accumulate in favor of
/// the protocol
/// NOTE: Calling this function when interests for the market are up to date with the current block
/// and index is not, will use the wrong interest rate to update the index.
/// NOTE: This function should not be used when calculating how much scaled amount is getting
/// burned from given underlying withdraw amount. In that case, all math should be done in underlying
/// amounts then get scaled back again
pub fn get_scaled_liquidity_amount(
    amount: Uint128,
    market: &Market,
    timestamp: u64,
) -> Result<Uint128, MarsError> {
    compute_scaled_amount(
        amount,
        get_updated_liquidity_index(market, timestamp)?,
        ScalingOperation::Truncate,
    )
}

/// Get underlying liquidity amount from a scaled amount, a Market and timestamp in seconds
/// Liquidity amounts are always truncated to make sure rounding errors accumulate in favor of
/// the protocol
/// NOTE: Calling this function when interests for the market are up to date with the current block
/// and index is not, will use the wrong interest rate to update the index.
pub fn get_underlying_liquidity_amount(
    amount_scaled: Uint128,
    market: &Market,
    timestamp: u64,
) -> Result<Uint128, MarsError> {
    compute_underlying_amount(
        amount_scaled,
        get_updated_liquidity_index(market, timestamp)?,
        ScalingOperation::Truncate,
    )
}

/// Get scaled borrow amount from an underlying amount, a Market and timestamp in seconds
/// Debt amounts are always ceiled to make sure rounding errors accumulate in favor of
/// the protocol
/// NOTE: Calling this function when interests for the market are up to date with the current block
/// and index is not, will use the wrong interest rate to update the index.
/// NOTE: This function should not be used when calculating how much scaled amount is getting
/// repaid from a sent underlying amount. In that case, all math should be done in underlying
/// amounts then get scaled back again
pub fn get_scaled_debt_amount(
    amount: Uint128,
    market: &Market,
    timestamp: u64,
) -> Result<Uint128, MarsError> {
    compute_scaled_amount(
        amount,
        get_updated_borrow_index(market, timestamp)?,
        ScalingOperation::Ceil,
    )
}

/// Get underlying borrow amount from a scaled amount, a Market and timestamp in seconds
/// Debt amounts are always ceiled so as for rounding errors to accumulate in favor of
/// the protocol
/// NOTE: Calling this function when interests for the market are up to date with the current block
/// and index is not, will use the wrong interest rate to update the index.
pub fn get_underlying_debt_amount(
    amount_scaled: Uint128,
    market: &Market,
    timestamp: u64,
) -> Result<Uint128, MarsError> {
    compute_underlying_amount(
        amount_scaled,
        get_updated_borrow_index(market, timestamp)?,
        ScalingOperation::Ceil,
    )
}

pub enum ScalingOperation {
    Truncate,
    Ceil,
}

/// Scales the amount dividing by an index in order to compute interest rates. Before dividing,
/// the value is multiplied by SCALING_FACTOR for greater precision.
/// Example:
/// Current index is 10. We deposit 6.123456 OSMO (6123456 uosmo). Scaled amount will be
/// 6123456 / 10 = 612345 so we loose some precision. In order to avoid this situation
/// we scale the amount by SCALING_FACTOR.
pub fn compute_scaled_amount(
    amount: Uint128,
    index: Decimal,
    scaling_operation: ScalingOperation,
) -> Result<Uint128, MarsError> {
    // Scale by SCALING_FACTOR to have better precision
    let scaled_amount = amount.checked_mul(SCALING_FACTOR)?;
    match scaling_operation {
        ScalingOperation::Truncate => Ok(scaled_amount.checked_div_floor(index)?),
        ScalingOperation::Ceil => Ok(scaled_amount.checked_div_ceil(index)?),
    }
}

/// Descales the amount introduced by `get_scaled_amount`, returning the underlying amount.
/// As interest rate is accumulated the index used to descale the amount should be bigger than the one used to scale it.
pub fn compute_underlying_amount(
    scaled_amount: Uint128,
    index: Decimal,
    scaling_operation: ScalingOperation,
) -> Result<Uint128, MarsError> {
    // Multiply scaled amount by decimal (index)
    let before_scaling_factor = scaled_amount * index;

    // Descale by SCALING_FACTOR which is introduced when scaling the amount
    match scaling_operation {
        ScalingOperation::Truncate => Ok(before_scaling_factor.checked_div(SCALING_FACTOR)?),
        ScalingOperation::Ceil => {
            let scaling_factor_dec = Decimal::from_ratio(SCALING_FACTOR, Uint128::one());
            Ok(before_scaling_factor.checked_div_ceil(scaling_factor_dec)?)
        }
    }
}

/// Return applied interest rate for borrow index according to passed blocks
/// NOTE: Calling this function when interests for the market are up to date with the current block
/// and index is not, will use the wrong interest rate to update the index.
pub fn get_updated_borrow_index(market: &Market, timestamp: u64) -> StdResult<Decimal> {
    if market.indexes_last_updated < timestamp {
        let time_elapsed = timestamp - market.indexes_last_updated;

        if !market.borrow_rate.is_zero() {
            let updated_index = calculate_applied_linear_interest_rate(
                market.borrow_index,
                market.borrow_rate,
                time_elapsed,
            );
            return updated_index;
        }
    }

    Ok(market.borrow_index)
}

/// Return applied interest rate for liquidity index according to passed blocks
/// NOTE: Calling this function when interests for the market are up to date with the current block
/// and index is not, will use the wrong interest rate to update the index.
pub fn get_updated_liquidity_index(market: &Market, timestamp: u64) -> StdResult<Decimal> {
    if market.indexes_last_updated > timestamp {
        return Err(StdError::generic_err(
            format!("Cannot compute updated liquidity index for a timestamp: {} smaller than last updated timestamp for market: {}", timestamp, market.indexes_last_updated)
        ));
    }

    if market.indexes_last_updated < timestamp {
        let time_elapsed = timestamp - market.indexes_last_updated;

        if !market.liquidity_rate.is_zero() {
            let updated_index = calculate_applied_linear_interest_rate(
                market.liquidity_index,
                market.liquidity_rate,
                time_elapsed,
            );
            return updated_index;
        }
    }

    Ok(market.liquidity_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulated_index_calculation() {
        let index = Decimal::from_ratio(1u128, 10u128);
        let rate = Decimal::from_ratio(2u128, 10u128);
        let time_elapsed = 15768000; // half a year
        let accumulated =
            calculate_applied_linear_interest_rate(index, rate, time_elapsed).unwrap();

        assert_eq!(accumulated, Decimal::from_ratio(11u128, 100u128));
    }

    #[test]
    fn liquidity_and_debt_rounding() {
        let start = Uint128::from(100_000_000_000_u128);
        let market = Market {
            liquidity_index: Decimal::from_ratio(3_u128, 1_u128),
            borrow_index: Decimal::from_ratio(3_u128, 1_u128),
            indexes_last_updated: 1,
            ..Default::default()
        };

        let scaled_amount_liquidity = get_scaled_liquidity_amount(start, &market, 1).unwrap();
        let scaled_amount_debt = get_scaled_debt_amount(start, &market, 1).unwrap();
        assert_eq!(Uint128::from(33_333_333_333_333_333_u128), scaled_amount_liquidity);
        assert_eq!(Uint128::from(33_333_333_333_333_334_u128), scaled_amount_debt);

        let back_to_underlying_liquidity =
            get_underlying_liquidity_amount(scaled_amount_liquidity, &market, 1).unwrap();
        let back_to_underlying_debt =
            get_underlying_debt_amount(scaled_amount_debt, &market, 1).unwrap();
        assert_eq!(Uint128::from(99_999_999_999_u128), back_to_underlying_liquidity);
        assert_eq!(Uint128::from(100_000_000_001_u128), back_to_underlying_debt);
    }
}
