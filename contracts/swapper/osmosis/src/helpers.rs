use std::{collections::HashSet, hash::Hash};

/// Build a hashset from array data
pub(crate) fn hashset<T: Eq + Clone + Hash>(data: &[T]) -> HashSet<T> {
    data.iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hashset() {
        let data = vec![1, 2, 3, 4, 5];
        let set = hashset(&data);
        assert_eq!(set.len(), 5);
        assert!(set.contains(&1));
        assert!(set.contains(&2));
        assert!(set.contains(&3));
        assert!(set.contains(&4));
        assert!(set.contains(&5));
    }
}
