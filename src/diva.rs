use crate::y_fast_trie::YFastTrie;
use crate::infix_store::InfixStore;
use crate::utils::longest_common_prefix_length;


const BASE_IMPLICIT_SIZE: u32 = 10;  

pub struct Diva {
    y_fast_trie: YFastTrie,
    target_size: usize,
    fpr: f64,
    remainder_size: u8  
}

impl Diva {
    pub fn new(target_size: usize, fpr: f64) -> Self {
        let remainder_size = Self::choose_remainder_size(target_size, fpr);
        const NO_LEVELS: usize = 64;
        Self {
            y_fast_trie: YFastTrie::new(NO_LEVELS),
            target_size,
            fpr,
            remainder_size
        }
    }

    pub fn new_with_keys(keys: &[u64], target_size: usize, fpr: f64) -> Self {
        let remainder_size = Self::choose_remainder_size(target_size, fpr);
        let mut sorted_keys = keys.to_vec();
        sorted_keys.sort();
        sorted_keys.dedup();

        // sample every target_size keys
        let sampled_keys = sorted_keys.iter().step_by(target_size).map(|k| *k).collect::<Vec<_>>();

        // TODO: make this dynamic based on the key length
        const NO_LEVELS: usize = 64;
        let mut y_fast_trie = YFastTrie::new_with_keys(&sampled_keys, NO_LEVELS);

        // for each pair of consecutive samples, extract infixes from intermediate keys
        for i in 0..sampled_keys.len().saturating_sub(1) {
            let predecessor = sampled_keys[i];
            let successor = sampled_keys[i + 1];

            // compute extraction parameters from boundary keys
            let (shared_prefix_len, redundant_bits, quotient_bits) =
                Self::get_shared_ignore_implicit_size(&predecessor, &successor, false);

            // find intermediate keys between these samples (skip the sample itself)
            let start_idx = i * target_size + 1;
            let end_idx = ((i + 1) * target_size).min(sorted_keys.len());

            // extract infixes from intermediate keys
            let mut infixes = Vec::new();
            for j in start_idx..end_idx {
                let key = sorted_keys[j];
                let key_msb = Self::get_msb(&predecessor, &key);
                let infix = Self::extract_partial_key(
                    key,
                    shared_prefix_len,
                    redundant_bits,
                    quotient_bits,
                    remainder_size,
                    key_msb,
                );
                infixes.push(infix);
            }

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
            remainder_size
        }
    }

    /// compute redundant bits after first differing bit
    /// redundant bits are consecutive bits with opposite patterns in pred/succ
    /// that can be reconstructed knowing the key is in this range
    fn compute_redundant_bits(key_1: u64, key_2: u64, shared_prefix_len: u8) -> u8 {
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
    fn get_shared_ignore_implicit_size(
        key_1: &u64,
        key_2: &u64,
        use_redundant_bits: bool
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
        let shift = remaining_bits - BASE_IMPLICIT_SIZE as u8;
        let quotient_1 = (key_1 >> shift) & ((1u64 << BASE_IMPLICIT_SIZE) - 1);
        let quotient_2 = (key_2 >> shift) & ((1u64 << BASE_IMPLICIT_SIZE) - 1);

        // TODO: check if there is a better heuristic for the quotient size
        // add 1 bit if range is sparse (uses < 50% of quotient space)
        let range_size = quotient_2 - quotient_1 + 1;
        let quotient_bits = if 2 * range_size < (1u64 << BASE_IMPLICIT_SIZE) {
            (BASE_IMPLICIT_SIZE + 1).min(remaining_bits as u32) as u8
        } else {
            BASE_IMPLICIT_SIZE as u8
        };

        (shared, redundant_bits, quotient_bits)
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
    fn extract_partial_key(
        key: u64,
        shared_prefix_len: u8,
        redundant_bits: u8,
        quotient_bits: u8,
        remainder_bits: u8,
        msb: u8,
    ) -> u64 {
        // position where extraction starts (after shared + first_diff + redundant)
        let start_bit = shared_prefix_len + 1 + redundant_bits;

        if start_bit >= 64 {
            return msb as u64;
        }

        // extract quotient + remainder bits
        let remaining_bits = 64 - start_bit;
        let bits_to_extract = (quotient_bits + remainder_bits).min(remaining_bits);

        if bits_to_extract == 0 {
            return msb as u64;
        }

        let shift_amount = 64 - start_bit - bits_to_extract;
        let extracted = (key >> shift_amount) & ((1u64 << bits_to_extract) - 1);

        // combine: [MSB: 1 bit][quotient: quotient_bits][remainder: remainder_bits]
        let result = ((msb as u64) << (quotient_bits + remainder_bits)) | extracted;

        result
    }

    /// get MSB (first differing bit) between predecessor and successor
    fn get_msb(key_1: &u64, key_2: &u64) -> u8 {
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

}