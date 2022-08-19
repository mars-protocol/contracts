use std::collections::HashSet;
use std::hash::Hash;

/// Build a hashset from array data
pub(crate) fn hashset<T: Eq + Clone + Hash>(data: &[T]) -> HashSet<T> {
    data.iter().cloned().collect()
}
