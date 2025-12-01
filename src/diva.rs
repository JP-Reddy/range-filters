use crate::Key;
use crate::U64_BITS;
use crate::infix_store::InfixStore;
use crate::utils::longest_common_prefix_length;
use crate::y_fast_trie::YFastTrie;
use std::fmt;

const BASE_IMPLICIT_SIZE: u32 = 10;

/// Diva range filter
///
/// # Arguments
/// * `y_fast_trie` - Y-Fast Trie
/// * `target_size` - Target size
/// * `fpr` - False positive rate
/// * `remainder_size` - Remainder size
///
/// # Example
/// ```rust
/// use range_filters::diva::Diva;
/// let keys = vec![1, 2, 3, 4, 5];
/// let target_size = 1024;
/// let fpr = 0.01;
/// let diva = Diva::new_with_keys(&keys, target_size, fpr);
/// ```
///
/// # Returns
/// * `Diva` - Diva range filter
pub struct Diva {
    y_fast_trie: YFastTrie,
    target_size: usize,
    fpr: f64,
    remainder_size: u8,
}

impl Diva {
    pub fn new(target_size: usize, fpr: f64) -> Self {
        let remainder_size = Self::choose_remainder_size(target_size, fpr);
        const NO_LEVELS: usize = 64;
        Self {
            y_fast_trie: YFastTrie::new(NO_LEVELS),
            target_size,
            fpr,
            remainder_size,
        }
    }

    pub fn new_with_keys(keys: &[Key], target_size: usize, fpr: f64) -> Self {
        let remainder_size = Self::choose_remainder_size(target_size, fpr);
        let mut sorted_keys = keys.to_vec();
        sorted_keys.sort();
        sorted_keys.dedup();

        let mut sampled_keys: Vec<Key> = sorted_keys.iter().step_by(target_size).copied().collect();

        // ensure last key is sampled if not already
        if let Some(&last_key) = sorted_keys.last() {
            if sampled_keys.last() != Some(&last_key) {
                sampled_keys.push(last_key);
            }
        }

        // TODO: make this dynamic based on the key length
        const NO_LEVELS: usize = U64_BITS;
        let mut y_fast_trie = YFastTrie::new_with_keys(&sampled_keys, NO_LEVELS);

        // for each pair of consecutive samples, extract infixes from intermediate keys
        for i in 0..sampled_keys.len().saturating_sub(1) {
            let predecessor = sampled_keys[i];
            let successor = sampled_keys[i + 1];

            // compute extraction parameters from boundary keys
            let (shared_prefix_len, redundant_bits, quotient_bits) =
                Self::get_shared_ignore_implicit_size(&predecessor, &successor, false);

            // find intermediate keys between these samples excluding the samples themselves
            let intermediate_keys: Vec<Key> = sorted_keys
                .iter()
                .filter(|&&k| k > predecessor && k < successor)
                .copied()
                .collect();

            // extract infixes from intermediate keys
            let mut infixes = Vec::new();
            println!("Processing keys between {} and {} (shared={}, redundant={}, quotient={})",
                     predecessor, successor, shared_prefix_len, redundant_bits, quotient_bits);
            for key in intermediate_keys {
                let key_msb = Self::get_msb(&predecessor, &key);
                let infix = Self::extract_partial_key(
                    key,
                    shared_prefix_len,
                    redundant_bits,
                    quotient_bits,
                    remainder_size,
                    key_msb,
                );
                println!("Key {} ({:064b}) -> infix {} ({:064b}) (MSB: {})",
                         key, key, infix, infix, key_msb);
                infixes.push(infix);
            }
            println!("Infixes before InfixStore creation: {:?}", infixes);

            // create InfixStore and attach to predecessor sample
            if !infixes.is_empty() {
                let infix_store = InfixStore::new_with_infixes(&infixes, remainder_size);
                y_fast_trie.set_infix_store(predecessor, infix_store);
            }
        }

        Self {
            y_fast_trie,
            target_size,
            fpr,
            remainder_size,
        }
    }

    /// compute redundant bits after first differing bit
    /// redundant bits are consecutive bits with opposite patterns in pred/succ
    /// that can be reconstructed knowing the key is in this range
    fn compute_redundant_bits(key_1: Key, key_2: Key, shared_prefix_len: u8) -> u8 {
        if shared_prefix_len >= 63 {
            return 0;
        }

        let mut redundant_bits = 0u8;

        // start after shared prefix + 1 (skip first differing bit)
        let start_pos = shared_prefix_len + 1;

        for bit_pos in start_pos..64 {
            let shift = 63 - bit_pos;
            let bit_1 = (key_1 >> shift) & 1;
            let bit_2 = (key_2 >> shift) & 1;

            // redundant if pred has 0 and succ has 1 (opposite of first diff bit)
            if bit_1 == 0 && bit_2 == 1 {
                redundant_bits += 1;
            } else {
                break; // stop at first non-redundant bit
            }
        }

        redundant_bits
    }

    /// compute shared prefix, redundant bits, and quotient size
    /// returns: (shared_prefix_len, redundant_bits, quotient_bits)
    pub fn get_shared_ignore_implicit_size(
        key_1: &Key,
        key_2: &Key,
        use_redundant_bits: bool,
    ) -> (u8, u8, u8) {
        // step 1: find shared prefix length (LCP)
        let shared = longest_common_prefix_length(*key_1, *key_2) as u8;

        // step 2: compute redundant bits
        let redundant_bits = if use_redundant_bits {
            Self::compute_redundant_bits(*key_1, *key_2, shared)
        } else {
            0
        };

        // step 3: compute quotient size or aka implicit bits
        let bits_used = shared + 1 + redundant_bits; // shared + first_diff + redundant

        if bits_used >= 64 {
            return (shared, redundant_bits, 0);
        }

        let remaining_bits = 64 - bits_used;

        // try to use BASE_IMPLICIT_SIZE quotient bits
        if remaining_bits < BASE_IMPLICIT_SIZE as u8 {
            return (shared, redundant_bits, remaining_bits);
        }

        // extract quotient bits from both keys to check sparsity
        // let shift = remaining_bits - BASE_IMPLICIT_SIZE as u8;
        // let quotient_1 = (key_1 >> shift) & ((1u64 << BASE_IMPLICIT_SIZE) - 1);
        // let quotient_2 = (key_2 >> shift) & ((1u64 << BASE_IMPLICIT_SIZE) - 1);

        // TODO: check if there is a better heuristic for the quotient size
        // add 1 bit if range is sparse (uses < 50% of quotient space)
        // let range_size = quotient_2 - quotient_1 + 1;
        // let quotient_bits = if 2 * range_size < (1u64 << BASE_IMPLICIT_SIZE) {
        //     (BASE_IMPLICIT_SIZE + 1).min(remaining_bits as u32) as u8
        // } else {
        //     BASE_IMPLICIT_SIZE as u8
        // };

        (shared, redundant_bits, BASE_IMPLICIT_SIZE as u8)
    }

    /// extract partial key (infix) from a full key
    /// returns: MSB | quotient_bits | remainder_bits
    ///
    /// # Arguments
    /// * `key` - The full key to extract from
    /// * `shared_prefix_len` - Number of shared prefix bits to skip
    /// * `redundant_bits` - Number of redundant bits to skip
    /// * `quotient_bits` - Number of quotient bits to extract (implicit)
    /// * `remainder_bits` - Number of remainder bits to extract (explicit)
    /// * `msb` - The first differing bit (0 or 1)
    pub fn extract_partial_key(
        key: Key,
        shared_prefix_len: u8,
        redundant_bits: u8,
        quotient_bits: u8,
        remainder_bits: u8,
        msb: u8,
    ) -> Key {
        // position where extraction starts (after shared + first_diff + redundant)
        let start_bit = shared_prefix_len + 1 + redundant_bits;

        if start_bit >= 64 {
            return msb as Key;
        }

        // extract quotient + remainder bits
        let remaining_bits = 64 - start_bit;
        let bits_to_extract = (quotient_bits + remainder_bits).min(remaining_bits);

        if bits_to_extract == 0 {
            return msb as Key;
        }

        let shift_amount = 64 - start_bit - bits_to_extract;
        let extracted = (key >> shift_amount) & ((1 << bits_to_extract) - 1);

        let total_bits_needed = quotient_bits + remainder_bits;
        let extracted = extracted << (total_bits_needed - bits_to_extract);

        // combine: [MSB: 1 bit][quotient: quotient_bits][remainder: remainder_bits]
        let result = ((msb as Key) << (quotient_bits + remainder_bits)) | extracted;

        result
    }

    /// get MSB (first differing bit) between predecessor and successor
    pub fn get_msb(key_1: &Key, key_2: &Key) -> u8 {
        let shared = longest_common_prefix_length(*key_1, *key_2);

        if shared >= 64 {
            return 0; // keys are identical
        }

        // extract bit at position 'shared' (first differing bit)
        let bit_pos = 63 - shared;
        ((key_1 >> bit_pos) & 1) as u8
    }

    /// calculate remainder size based on FPR
    /// FPR ≈ 2 / 2^remainder_size
    fn choose_remainder_size(_target_size: usize, fpr: f64) -> u8 {
        // remainder_size = log2(2/FPR) = log2(2) + log2(1/FPR) = 1 - log2(FPR)
        let remainder_size = (1.0 - fpr.log2()).ceil() as u8;
        remainder_size.max(4).min(16) // clamp between 4 and 16 bits
    }

    pub fn pretty_print(&self) {
        print!("{}", self);
    }

    /// Point lookup: check if a key exists in the filter
    /// Returns true if key might exist (with FPR), false if definitely doesn't exist
    pub fn contains(&self, key: Key) -> bool {
        // Step 1: Check Y-Fast Trie directly (samples)
        if self.y_fast_trie.contains(key) {
            return true;
        }

        // Step 2: Find predecessor sample to locate the appropriate InfixStore
        if let Some(predecessor_infix_store) = self.y_fast_trie.predecessor_infix_store(key) {
            // Step 3: Find predecessor and successor samples to get infix conversion params
            if let Some(predecessor_key) = self.y_fast_trie.predecessor(key) {
                if let Some(successor_key) = self.y_fast_trie.successor(key) {
                    println!("Querying key {}, pred: {}, succ: {}", key, predecessor_key, successor_key);
                    // Step 4: Query the InfixStore
                    if let Ok(store) = predecessor_infix_store.read() {
                        let result = store.point_query(key, predecessor_key, successor_key, self.remainder_size);
                        println!("InfixStore query result for key {}: {}", key, result);
                        return result;
                    }
                }
            }
        }

        println!("No predecessor InfixStore found for key {}", key);

        false
    }
}

impl fmt::Display for Diva {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "\n╔════════════════════════════════════════════════════════╗"
        )?;
        writeln!(
            f,
            "║                   DIVA RANGE FILTER                    ║"
        )?;
        writeln!(
            f,
            "╚════════════════════════════════════════════════════════╝"
        )?;

        // configuration
        writeln!(f, "\nConfiguration:")?;
        writeln!(f, "  Target size (T):      {}", self.target_size)?;
        writeln!(f, "  False positive rate:  {:.4}%", self.fpr * 100.0)?;
        writeln!(f, "  Remainder size:       {} bits", self.remainder_size)?;

        // stats
        writeln!(f, "\nStatistics:")?;
        writeln!(f, "  Total keys:           {}", self.y_fast_trie.len())?;
        writeln!(
            f,
            "  Sample count:         {}",
            self.y_fast_trie.sample_count()
        )?;
        let avg_bucket_size = if self.y_fast_trie.sample_count() > 0 {
            self.y_fast_trie.len() as f64 / self.y_fast_trie.sample_count() as f64
        } else {
            0.0
        };
        writeln!(f, "  Avg keys per bucket:  {:.1}", avg_bucket_size)?;

        // underlying Y-Fast Trie structure
        write!(f, "{}", self.y_fast_trie)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_choose_remainder_size() {
        // FPR = 1% -> remainder_size = 8
        assert_eq!(Diva::choose_remainder_size(1024, 0.01), 8);
        // FPR = 0.1% -> remainder_size = 11
        assert_eq!(Diva::choose_remainder_size(1024, 0.001), 11);
        assert_eq!(Diva::choose_remainder_size(1024, 0.1), 5);
    }

    #[test]
    fn test_get_msb() {
        // first differing bit is 0
        let key1 = 0b0000_0000_0000_0000u64;
        let key2 = 0b1111_1111_1111_1111u64;
        assert_eq!(Diva::get_msb(&key1, &key2), 0);

        // first differing bit is 1
        let key1 = 0b1000_0000_0000_0000u64 << 48;
        let key2 = 0b0111_1111_1111_1111u64 << 48;
        assert_eq!(Diva::get_msb(&key1, &key2), 1);
    }

    #[test]
    fn test_extraction_params() {
        let key1 = 0b0000_0000_1111_0000u64 << 48;
        let key2 = 0b0000_0000_1111_1111u64 << 48;

        let (shared, _redundant, quotient) =
            Diva::get_shared_ignore_implicit_size(&key1, &key2, false);

        assert_eq!(shared, 12); // 12 bits shared prefix
        // assert_eq!(redundant, 0);
        assert!(quotient >= 10); // at least 10 bits for quotient
    }

    #[test]
    fn test_construction_small_dataset() {
        // 100 keys, all fit in one sample
        let keys: Vec<u64> = (0..100).map(|i| i * 1000).collect();
        let diva = Diva::new_with_keys(&keys, 1024, 0.01);

        assert_eq!(diva.target_size, 1024);
        assert_eq!(diva.fpr, 0.01);
        assert_eq!(diva.remainder_size, 8);
    }

    #[test]
    fn test_construction_with_sampling() {
        // 5000 keys - should create ~5 samples
        let keys: Vec<u64> = (0..5000).map(|i| i as u64).collect();
        let target_size = 1024;
        let diva = Diva::new_with_keys(&keys, target_size, 0.01);

        // +1 because we sample the last key too
        let expected_samples = (keys.len() + target_size - 1) / target_size + 1;
        let actual_samples = diva.y_fast_trie.len();

        assert_eq!(actual_samples, expected_samples);
    }

    #[test]
    fn test_construction_single_sample() {
        // 500 keys < target_size -> only 1 sample
        let keys: Vec<u64> = (0..500).map(|i| i * 10).collect();
        let diva = Diva::new_with_keys(&keys, 1024, 0.01);

        assert_eq!(diva.y_fast_trie.sample_count(), 1);
    }

    #[test]
    fn test_point_query_simple() {
        // Simple test case with small numbers
        let keys: Vec<Key> = vec![100, 200, 300, 400, 500, 600, 700];
        let target_size = 1024; // This will create samples at 100, 400, 700
        let diva = Diva::new_with_keys(&keys, target_size, 0.01);

        println!("Created Diva with {} keys", keys.len());
        diva.pretty_print();

        // Test that all original keys are found
        for &key in &keys {
            println!("Testing key: {}", key);
            let found = diva.contains(key);
            assert!(found, "Key {} should be found", key);
        }

        // Test some keys that shouldn't exist
        assert!(!diva.contains(150), "Key 150 should not be found");
        assert!(!diva.contains(250), "Key 250 should not be found");
    }

    #[test]
    #[ignore]
    fn test_point_query() {
        // Create dataset with keys that will create multiple InfixStores
        let keys: Vec<Key> = (0..3000).map(|i| i * 100).collect();
        let target_size = 1000;
        let diva = Diva::new_with_keys(&keys, target_size, 0.01);

        // Test that all original keys are found
        for &key in &keys {
            assert!(diva.contains(key), "Key {} should be found", key);
        }

        // Test some keys that shouldn't exist
        assert!(!diva.contains(50), "Key 50 should not be found");
        assert!(!diva.contains(150), "Key 150 should not be found");
        assert!(!diva.contains(99999), "Key 99999 should not be found");
    }

    #[test]
    fn test_point_query_with_samples() {
        // Test that sampled keys are found via Y-Fast Trie directly
        let keys: Vec<Key> = vec![100, 200, 300, 400, 500];
        let target_size = 2;
        let diva = Diva::new_with_keys(&keys, target_size, 0.01);

        // Keys 100, 300, 500 should be samples
        // Keys 200, 400 should be in InfixStores
        for &key in &keys {
            assert!(diva.contains(key), "Key {} should be found", key);
        }
    }
}
