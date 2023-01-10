use cosmwasm_std::{ConversionOverflowError, Decimal, DivideByZeroError, Uint128};

use mars_math::CheckedMultiplyFractionError::{ConversionOverflow, DivideByZero};
use mars_math::{FractionMath, Fractional};

#[test]
fn div_floor_raises_with_zero() {
    let fraction = Fractional(Uint128::zero(), Uint128::new(21));
    assert_eq!(
        Uint128::new(123456).checked_div_floor(fraction),
        Err(DivideByZero(DivideByZeroError {
            operand: "2592576".to_string()
        })),
    );
}

#[test]
fn div_floor_does_nothing_with_one() {
    let fraction = Fractional(Uint128::one(), Uint128::one());
    let res = Uint128::new(123456).checked_div_floor(fraction).unwrap();
    assert_eq!(Uint128::new(123456), res)
}

#[test]
fn div_floor_rounds_down_with_normal_case() {
    let fraction = Fractional(5u128, 21u128);
    let res = Uint128::new(123456).checked_div_floor(fraction).unwrap(); // 518515.2
    assert_eq!(Uint128::new(518515), res)
}

#[test]
fn div_floor_does_not_round_on_even_divide() {
    let fraction = Fractional(5u128, 2u128);
    let res = Uint128::new(25).checked_div_floor(fraction).unwrap();

    assert_eq!(Uint128::new(10), res)
}

#[test]
fn div_floor_works_when_operation_temporarily_takes_above_max() {
    let fraction = Fractional(21u128, 8u128);
    let res = Uint128::MAX.checked_div_floor(fraction).unwrap(); // 129_631_377_874_643_224_176_523_659_974_006_937_697.1428
    assert_eq!(
        Uint128::new(129_631_377_874_643_224_176_523_659_974_006_937_697),
        res
    )
}

#[test]
fn div_floor_works_with_decimal() {
    let decimal = Decimal::from_ratio(21u128, 8u128);
    let res = Uint128::new(123456).checked_div_floor(decimal).unwrap(); // 47030.857
    assert_eq!(Uint128::new(47030), res)
}

#[test]
fn div_floor_works_with_decimal_evenly() {
    let res = Uint128::new(60)
        .checked_div_floor(Decimal::from_atomics(6u128, 0).unwrap())
        .unwrap();
    assert_eq!(res, Uint128::new(10));
}

#[test]
fn checked_div_floor_does_not_panic_on_overflow() {
    let fraction = Fractional(8u128, 21u128);
    assert_eq!(
        Uint128::MAX.checked_div_floor(fraction),
        Err(ConversionOverflow(ConversionOverflowError {
            source_type: "Uint256",
            target_type: "Uint128",
            value: "893241213167463466591358344508391555069".to_string()
        })),
    );
}
