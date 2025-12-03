use fastbloom::BloomFilter as FastBloomFilter;

use crate::Key;

pub struct BloomFilter {
    filter: FastBloomFilter,
    fpr: f64,
    num_keys: usize,
}

impl BloomFilter {
    /// Create a new Bloom Filter with the given keys and false positive rate.
    ///
    /// # Arguments
    /// * `keys` - A slice of keys to insert into the filter
    /// * `fpr` - Target false positive rate (e.g., 0.01 for 1%)
    ///
    /// # Returns
    /// A new `BloomFilter` instance containing all the provided keys
    pub fn new_with_keys(keys: &[Key], fpr: f64) -> Self {
        let num_keys = keys.len();

        let mut filter = FastBloomFilter::with_false_pos(fpr).expected_items(num_keys);
        for &key in keys {
            filter.insert(&key.to_string());
        }
        Self {
            filter,
            fpr,
            num_keys,
        }
    }

    /// Perform a point query to check if a key might exist in the filter.
    ///
    /// # Arguments
    /// * `key` - The key to search for
    ///
    /// # Returns
    /// * `true` if the key might exist (with false positive rate `fpr`)
    /// * `false` if the key definitely does not exist
    pub fn point_query(&self, key: Key) -> bool {
        self.filter.contains(&key.to_string())
    }

    /// Perform a range query to check if any key might exist in the given range [start, end] (inclusive).
    ///
    /// # Arguments
    /// * `start` - The start of the range (inclusive)
    /// * `end` - The end of the range (inclusive)
    ///
    /// # Returns
    /// * `true` if at least one key in the range might exist (with false positive rate `fpr`)
    /// * `false` if no keys in the range exist
    pub fn range_query(&self, start: Key, end: Key) -> bool {
        if start > end {
            return false;
        }

        for key in start..=end {
            if self.filter.contains(&key.to_string()) {
                return true;
            }
        }
        false
    }

    /// Get the configured false positive rate.
    ///
    /// # Returns
    /// The false positive rate (e.g., 0.01 for 1%)
    pub fn fpr(&self) -> f64 {
        self.fpr
    }

    /// Get the number of keys inserted into the filter.
    pub fn num_keys(&self) -> usize {
        self.num_keys
    }
}
