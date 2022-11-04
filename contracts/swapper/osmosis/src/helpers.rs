use cosmwasm_std::{Decimal, Uint128};
use std::collections::HashSet;
use std::hash::Hash;

/// Build a hashset from array data
pub(crate) fn hashset<T: Eq + Clone + Hash>(data: &[T]) -> HashSet<T> {
    data.iter().cloned().collect()
}

pub trait IntoUint128 {
    fn uint128(&self) -> Uint128;
}

impl IntoUint128 for Decimal {
    fn uint128(&self) -> Uint128 {
        *self * Uint128::new(1)
    }
}
