use cosmwasm_std::{Coin, Decimal, DecimalRangeExceeded, Uint128};

pub trait Stringify {
    fn to_string(&self) -> String;
}

pub trait Denoms {
    fn to_denoms(&self) -> Vec<&str>;
}

pub trait Coins {
    fn to_coins(&self) -> Vec<Coin>;
}

pub trait IntoUint128 {
    fn uint128(&self) -> Uint128;
}

impl IntoUint128 for Decimal {
    fn uint128(&self) -> Uint128 {
        *self * Uint128::new(1)
    }
}

pub trait IntoDecimal {
    fn to_dec(&self) -> Result<Decimal, DecimalRangeExceeded>;
}

impl IntoDecimal for Uint128 {
    fn to_dec(&self) -> Result<Decimal, DecimalRangeExceeded> {
        Decimal::from_atomics(*self, 0)
    }
}

impl IntoDecimal for u128 {
    fn to_dec(&self) -> Result<Decimal, DecimalRangeExceeded> {
        Decimal::from_atomics(*self, 0)
    }
}
