use grafite::{PairwiseIndependentHasher, RangeFilter};

use crate::Key;

pub struct GrafiteFilter {
    filter: RangeFilter,
    epsilon: f64,
    num_keys: usize,
}

impl GrafiteFilter {
    /// Create a new Grafite Range Filter with the given keys and epsilon (false positive rate).
    ///
    /// # Arguments
    /// * `keys` - A slice of keys to insert into the filter
    /// * `epsilon` - Target false positive rate (e.g., 0.01 for 1%)
    ///
    /// # Returns
    /// A new `GrafiteFilter` instance containing all the provided keys
    pub fn new_with_keys(keys: &[Key], epsilon: f64) -> Self {
        let num_keys = keys.len();

        // Calculate max_query_range from the keys
        let max_query_range = if keys.is_empty() {
            0
        } else {
            *keys.iter().max().unwrap_or(&0)
        };

        let hasher = PairwiseIndependentHasher::new(num_keys, epsilon, max_query_range)
            .expect("Invalid parameters for PairwiseIndependentHasher");

        let filter = RangeFilter::new(keys.iter().copied(), hasher);

        Self {
            filter,
            epsilon,
            num_keys,
        }
    }

    /// Perform a point query to check if a key might exist in the filter.
    ///
    /// # Arguments
    /// * `key` - The key to search for
    ///
    /// # Returns
    /// * `true` if the key might exist (with false positive rate `epsilon`)
    /// * `false` if the key definitely does not exist
    pub fn point_query(&self, key: Key) -> bool {
        self.filter.query(key..=key)
    }

    /// Perform a range query to check if any key might exist in the given range [start, end] (inclusive).
    ///
    /// # Arguments
    /// * `start` - The start of the range (inclusive)
    /// * `end` - The end of the range (inclusive)
    ///
    /// # Returns
    /// * `true` if at least one key in the range might exist (with false positive rate `epsilon`)
    /// * `false` if no keys in the range exist
    pub fn range_query(&self, start: Key, end: Key) -> bool {
        if start > end {
            return false;
        }
        self.filter.query(start..=end)
    }

    /// Get the configured false positive rate (epsilon).
    ///
    /// # Returns
    /// The false positive rate (e.g., 0.01 for 1%)
    pub fn fpr(&self) -> f64 {
        self.epsilon
    }

    /// Get the number of keys inserted into the filter.
    pub fn num_keys(&self) -> usize {
        self.num_keys
    }
}
