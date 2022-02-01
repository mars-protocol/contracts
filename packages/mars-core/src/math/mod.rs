pub mod decimal;

use cosmwasm_std::{StdResult, Uint128};

pub fn uint128_checked_div_with_ceil(
    numerator: Uint128,
    denominator: Uint128,
) -> StdResult<Uint128> {
    let mut result = numerator.checked_div(denominator)?;

    if numerator.checked_rem(denominator)? > Uint128::zero() {
        result += Uint128::from(1_u128);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
