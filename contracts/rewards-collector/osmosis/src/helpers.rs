use std::{collections::HashSet, hash::Hash};

/// Build a hashset from array data
pub fn hashset<T: Eq + Clone + Hash>(data: &[T]) -> HashSet<T> {
    data.iter().cloned().collect()
}
