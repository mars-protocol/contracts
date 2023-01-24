use std::convert::TryInto;

use cosmwasm_std::{
    CheckedFromRatioError, Decimal, Fraction, OverflowError, OverflowOperation, StdError,
    StdResult, Uint128, Uint256,
};

pub fn uint128_checked_div_with_ceil(
    numerator: Uint128,
    denominator: Uint128,
) -> StdResult<Uint128> {
    let mut result = numerator.checked_div(denominator)?;

    if !numerator.checked_rem(denominator)?.is_zero() {
        result += Uint128::from(1_u128);
    }

    Ok(result)
}

/// Divide 'a' by 'b'.
pub fn divide_decimal_by_decimal(a: Decimal, b: Decimal) -> StdResult<Decimal> {
    Decimal::checked_from_ratio(a.numerator(), b.numerator()).map_err(|e| match e {
        CheckedFromRatioError::Overflow => StdError::Overflow {
            source: OverflowError {
                operation: OverflowOperation::Mul,
                operand1: a.numerator().to_string(),
                operand2: a.denominator().to_string(),
            },
        },
        CheckedFromRatioError::DivideByZero => StdError::DivideByZero {
            source: cosmwasm_std::DivideByZeroError {
                operand: b.to_string(),
            },
        },
    })
}

/// Divide Uint128 by Decimal.
/// (Uint128 / numerator / denominator) is equal to (Uint128 * denominator / numerator).
pub fn divide_uint128_by_decimal(a: Uint128, b: Decimal) -> StdResult<Uint128> {
    // (Uint128 / numerator / denominator) is equal to (Uint128 * denominator / numerator).
    let numerator_u256 = a.full_mul(b.denominator());
    let denominator_u256 = Uint256::from(b.numerator());

    let result_u256 = numerator_u256 / denominator_u256;

    let result = result_u256.try_into()?;
    Ok(result)
}

/// Divide Uint128 by Decimal, rounding up to the nearest integer.
pub fn divide_uint128_by_decimal_and_ceil(a: Uint128, b: Decimal) -> StdResult<Uint128> {
    // (Uint128 / numerator / denominator) is equal to (Uint128 * denominator / numerator).
    let numerator_u256 = a.full_mul(b.denominator());
    let denominator_u256 = Uint256::from(b.numerator());

    let mut result_u256 = numerator_u256 / denominator_u256;

    if numerator_u256.checked_rem(denominator_u256)? > Uint256::zero() {
        result_u256 += Uint256::from(1_u32);
    }

    let result = result_u256.try_into()?;
    Ok(result)
}

/// Multiply Uint128 by Decimal, rounding up to the nearest integer.
pub fn multiply_uint128_by_decimal_and_ceil(a: Uint128, b: Decimal) -> StdResult<Uint128> {
    let numerator_u256 = a.full_mul(b.numerator());
    let denominator_u256 = Uint256::from(b.denominator());

    let mut result_u256 = numerator_u256 / denominator_u256;

    if numerator_u256.checked_rem(denominator_u256)? > Uint256::zero() {
        result_u256 += Uint256::from(1_u32);
    }

    let result = result_u256.try_into()?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use cosmwasm_std::{ConversionOverflowError, OverflowOperation};

    use super::*;

    const DECIMAL_FRACTIONAL: Uint128 = Uint128::new(1_000_000_000_000_000_000u128); // 1*10**18
    const DECIMAL_FRACTIONAL_SQUARED: Uint128 =
        Uint128::new(1_000_000_000_000_000_000_000_000_000_000_000_000u128); // (1*10**18)**2 = 1*10**36

    #[test]
    fn test_uint128_checked_div_with_ceil() {
        let a = Uint128::new(120u128);
        let b = Uint128::zero();
        uint128_checked_div_with_ceil(a, b).unwrap_err();

        let a = Uint128::new(120u128);
        let b = Uint128::new(60_u128);
        let c = uint128_checked_div_with_ceil(a, b).unwrap();
        assert_eq!(c, Uint128::new(2u128));

        let a = Uint128::new(120u128);
        let b = Uint128::new(119_u128);
        let c = uint128_checked_div_with_ceil(a, b).unwrap();
        assert_eq!(c, Uint128::new(2u128));

        let a = Uint128::new(120u128);
        let b = Uint128::new(120_u128);
        let c = uint128_checked_div_with_ceil(a, b).unwrap();
        assert_eq!(c, Uint128::new(1u128));

        let a = Uint128::new(120u128);
        let b = Uint128::new(121_u128);
        let c = uint128_checked_div_with_ceil(a, b).unwrap();
        assert_eq!(c, Uint128::new(1u128));

        let a = Uint128::zero();
        let b = Uint128::new(121_u128);
        let c = uint128_checked_div_with_ceil(a, b).unwrap();
        assert_eq!(c, Uint128::zero());
    }

    #[test]
    fn checked_decimal_division() {
        let a = Decimal::from_ratio(99988u128, 100u128);
        let b = Decimal::from_ratio(24997u128, 100u128);
        let c = divide_decimal_by_decimal(a, b).unwrap();
        assert_eq!(c, Decimal::from_str("4.0").unwrap());

        let a = Decimal::from_ratio(123456789u128, 1000000u128);
        let b = Decimal::from_ratio(33u128, 1u128);
        let c = divide_decimal_by_decimal(a, b).unwrap();
        assert_eq!(c, Decimal::from_str("3.741114818181818181").unwrap());

        let a = Decimal::MAX;
        let b = Decimal::MAX;
        let c = divide_decimal_by_decimal(a, b).unwrap();
        assert_eq!(c, Decimal::one());

        // Note: DivideByZeroError is not public so we just check if dividing by zero returns error
        let a = Decimal::one();
        let b = Decimal::zero();
        divide_decimal_by_decimal(a, b).unwrap_err();

        let a = Decimal::MAX;
        let b = Decimal::from_ratio(1u128, DECIMAL_FRACTIONAL);
        let res_error = divide_decimal_by_decimal(a, b).unwrap_err();
        assert_eq!(
            res_error,
            OverflowError::new(OverflowOperation::Mul, Uint128::MAX, DECIMAL_FRACTIONAL).into()
        );
    }

    #[test]
    fn test_divide_uint128_by_decimal() {
        let a = Uint128::new(120u128);
        let b = Decimal::from_ratio(120u128, 15u128);
        let c = divide_uint128_by_decimal(a, b).unwrap();
        assert_eq!(c, Uint128::new(15u128));

        let a = Uint128::new(DECIMAL_FRACTIONAL.u128());
        let b = Decimal::from_ratio(DECIMAL_FRACTIONAL.u128(), 1u128);
        let c = divide_uint128_by_decimal(a, b).unwrap();
        assert_eq!(c, Uint128::new(1u128));

        let a = Uint128::new(DECIMAL_FRACTIONAL.u128());
        let b = Decimal::from_ratio(1u128, DECIMAL_FRACTIONAL.u128());
        let c = divide_uint128_by_decimal(a, b).unwrap();
        assert_eq!(c, Uint128::new(DECIMAL_FRACTIONAL_SQUARED.u128()));

        let a = Uint128::MAX;
        let b = Decimal::one();
        let c = divide_uint128_by_decimal(a, b).unwrap();
        assert_eq!(c, Uint128::MAX);

        let a = Uint128::new(1_000_000_000_000_000_000);
        let b = Decimal::from_ratio(1u128, DECIMAL_FRACTIONAL);
        let c = divide_uint128_by_decimal(a, b).unwrap();
        assert_eq!(c, Uint128::new(1_000_000_000_000_000_000_000_000_000_000_000_000));

        // Division is truncated
        let a = Uint128::new(100);
        let b = Decimal::from_ratio(3u128, 1u128);
        let c = divide_uint128_by_decimal(a, b).unwrap();
        assert_eq!(c, Uint128::new(33));

        let a = Uint128::new(75);
        let b = Decimal::from_ratio(100u128, 1u128);
        let c = divide_uint128_by_decimal(a, b).unwrap();
        assert_eq!(c, Uint128::new(0));

        // Overflow
        let a = Uint128::MAX;
        let b = Decimal::from_ratio(1_u128, 10_u128);
        let res_error = divide_uint128_by_decimal(a, b).unwrap_err();
        assert_eq!(
            res_error,
            ConversionOverflowError::new(
                "Uint256",
                "Uint128",
                "3402823669209384634633746074317682114550"
            )
            .into()
        );
    }

    #[test]
    fn test_divide_uint128_by_decimal_and_ceil() {
        let a = Uint128::new(120u128);
        let b = Decimal::from_ratio(120u128, 15u128);
        let c = divide_uint128_by_decimal_and_ceil(a, b).unwrap();
        assert_eq!(c, Uint128::new(15u128));

        let a = Uint128::new(DECIMAL_FRACTIONAL.u128());
        let b = Decimal::from_ratio(DECIMAL_FRACTIONAL.u128(), 1u128);
        let c = divide_uint128_by_decimal_and_ceil(a, b).unwrap();
        assert_eq!(c, Uint128::new(1u128));

        let a = Uint128::new(DECIMAL_FRACTIONAL.u128());
        let b = Decimal::from_ratio(1u128, DECIMAL_FRACTIONAL.u128());
        let c = divide_uint128_by_decimal_and_ceil(a, b).unwrap();
        assert_eq!(c, Uint128::new(DECIMAL_FRACTIONAL_SQUARED.u128()));

        let a = Uint128::MAX;
        let b = Decimal::one();
        let c = divide_uint128_by_decimal_and_ceil(a, b).unwrap();
        assert_eq!(c, Uint128::MAX);

        let a = Uint128::new(1_000_000_000_000_000_000);
        let b = Decimal::from_ratio(1u128, DECIMAL_FRACTIONAL);
        let c = divide_uint128_by_decimal_and_ceil(a, b).unwrap();
        assert_eq!(c, Uint128::new(1_000_000_000_000_000_000_000_000_000_000_000_000));

        // Division is rounded up
        let a = Uint128::new(100);
        let b = Decimal::from_ratio(3u128, 1u128);
        let c = divide_uint128_by_decimal_and_ceil(a, b).unwrap();
        assert_eq!(c, Uint128::new(34));

        let a = Uint128::new(75);
        let b = Decimal::from_ratio(100u128, 1u128);
        let c = divide_uint128_by_decimal_and_ceil(a, b).unwrap();
        assert_eq!(c, Uint128::new(1));

        // Overflow
        let a = Uint128::MAX;
        let b = Decimal::from_ratio(1_u128, 10_u128);
        let res_error = divide_uint128_by_decimal_and_ceil(a, b).unwrap_err();
        assert_eq!(
            res_error,
            ConversionOverflowError::new(
                "Uint256",
                "Uint128",
                "3402823669209384634633746074317682114550"
            )
            .into()
        );
    }
}
