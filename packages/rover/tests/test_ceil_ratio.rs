use cosmwasm_std::{CheckedMultiplyRatioError, Uint128};
use mars_rover::math::CeilRatio;

const MAX_UINT128_SIZE: u128 = 340_282_366_920_938_463_463_374_607_431_768_211_455;

#[test]
fn test_divide_by_zero() {
    let err = Uint128::new(123)
        .multiply_ratio_ceil(Uint128::new(12), Uint128::zero())
        .unwrap_err();
    assert_eq!(err, CheckedMultiplyRatioError::DivideByZero)
}

#[test]
fn test_result_exceeds_128_bit_capacity() {
    let err = Uint128::new(MAX_UINT128_SIZE)
        .multiply_ratio_ceil(Uint128::new(2), Uint128::new(1))
        .unwrap_err();
    assert_eq!(err, CheckedMultiplyRatioError::Overflow)
}

#[test]
fn test_works_with_zero() {
    let res = Uint128::zero()
        .multiply_ratio_ceil(Uint128::new(1), Uint128::new(10))
        .unwrap();
    assert_eq!(res, Uint128::zero())
}

#[test]
fn test_works_with_one() {
    let res = Uint128::one()
        .multiply_ratio_ceil(Uint128::new(1), Uint128::new(10))
        .unwrap();
    assert_eq!(res, Uint128::one())
}

#[test]
fn test_not_increment_if_divides_cleanly() {
    // 56088 / 123 = 456
    let res = Uint128::new(56088)
        .multiply_ratio_ceil(Uint128::new(1), Uint128::new(123))
        .unwrap();
    assert_eq!(res, Uint128::new(456))
}

#[test]
fn test_rounds_up() {
    // 56000 / 123 = 455.28455284
    let res = Uint128::new(56000)
        .multiply_ratio_ceil(Uint128::new(1), Uint128::new(123))
        .unwrap();
    assert_eq!(res, Uint128::new(456))
}
