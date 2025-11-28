use crate::U64_BITS;
use crate::bitmap::{get_bit, rank, set_bit};
use std::fmt;

const TARGET_SIZE: u16 = 1024;
// const LOAD_FACTOR: f64 = 0.95;
const SIZE_GRADE_COUNT: usize = 31;
// const DEFAULT_SIZE_GRADE: u8 = 14;

// precomputed number of slots for each size grade
// size grades 0-30
// grade 14 is neutral - 1024 slots
const SCALED_SIZES: [u16; 31] = [
    463, 488, 514, 541, 570, 600, 632, 666, 701, 738, 777, 818, 861, 907, 1024, 1078, 1135, 1195,
    1258, 1325, 1395, 1469, 1547, 1629, 1715, 1806, 1901, 2002, 2108, 2219, 2326,
];

/// Memory layout of data:
/// [popcounts: 64 bits] [occupieds: TARGET_SIZE bits]
/// [runends: num_slots bits] [slots: num_slots * remainder_size bits]
/// popcounts: 32 bits for occupieds and 32 bits for runends
#[derive(Debug, Default)]
pub struct InfixStore {
    elem_count: u16,
    size_grade: u8, // decides the number of slots in the infix store
    remainder_size: u8,
    data: Vec<u64>,
}

impl InfixStore {
    /// Create a new InfixStore from sorted extracted infixes
    ///
    /// # Arguments
    /// * `infixes` - Sorted list of extracted partial keys (quotient|remainder)
    /// * `remainder_size` - Number of bits for remainder part
    pub fn new_with_infixes(infixes: &[u64], remainder_size: u8) -> Self {
        // step 1: determine size_grade based on number of elements
        let size_grade = Self::choose_size_grade(infixes.len());
        let num_slots = SCALED_SIZES[size_grade as usize];

        // step 2: calculate total data size needed
        // [popcounts: 64 bits] [occupieds: TARGET_SIZE bits]
        // [runends: num_slots bits] [slots: num_slots * remainder_size bits]
        let popcounts_words = 1;
        let occupieds_words = (TARGET_SIZE as usize + U64_BITS - 1) / U64_BITS;
        let runends_words = (num_slots as usize + U64_BITS - 1) / U64_BITS;
        let slots_bits = num_slots as usize * remainder_size as usize;
        let slots_words = (slots_bits + U64_BITS - 1) / U64_BITS;

        let total_words = popcounts_words + occupieds_words + runends_words + slots_words;
        let mut data = vec![0u64; total_words];

        if infixes.is_empty() {
            return Self {
                elem_count: 0,
                size_grade,
                remainder_size,
                data,
            };
        }

        // step 3: load infixes in the infix store
        Self::load_infixes_to_store(&mut data, infixes, remainder_size, num_slots);

        Self {
            elem_count: infixes.len() as u16,
            size_grade,
            remainder_size,
            data,
        }
    }

    /// choose appropriate size_grade based on number of elements
    fn choose_size_grade(num_elements: usize) -> u8 {
        for grade in 0..SIZE_GRADE_COUNT {
            if SCALED_SIZES[grade] >= num_elements as u16 {
                return grade as u8;
            }
        }
        (SIZE_GRADE_COUNT - 1) as u8
    }

    /// load sorted infixes into the infix store
    fn load_infixes_to_store(
        data: &mut [u64],
        infixes: &[u64],
        remainder_size: u8,
        num_slots: u16,
    ) {
        let occupieds_start = 1;
        let occupieds_words = (TARGET_SIZE as usize + U64_BITS - 1) / U64_BITS;
        let runends_start = occupieds_start + occupieds_words;
        let runends_words = (num_slots as usize + U64_BITS - 1) / U64_BITS;
        let slots_start = runends_start + runends_words;
        let slots_words = (num_slots as usize * remainder_size as usize + U64_BITS - 1) / U64_BITS;

        let mut slot_pos = 0;
        let mut prev_quotient = None;

        for infix in infixes {
            let (quotient, remainder) = Self::split_infix(*infix, remainder_size);

            // set quotient bit in occupieds bitmap
            let occupieds_slice = &mut data[occupieds_start..occupieds_start + occupieds_words];
            set_bit(occupieds_slice, quotient as usize);

            let is_last_in_run = prev_quotient.is_some() && prev_quotient.unwrap() != quotient;

            if is_last_in_run {
                // mark end of previous run
                let runends_slice = &mut data[runends_start..runends_start + runends_words];
                set_bit(runends_slice, slot_pos - 1);
            }

            // write remainder to slot
            let slots_slice = &mut data[slots_start..slots_start + slots_words];
            Self::write_slot(slots_slice, slot_pos, remainder, remainder_size);

            prev_quotient = Some(quotient);
            slot_pos += 1;
        }

        if slot_pos > 0 {
            let runends_slice = &mut data[runends_start..runends_start + runends_words];
            set_bit(runends_slice, slot_pos - 1);
        }

        Self::compute_popcounts(data, occupieds_start, runends_start, num_slots);
    }

    /// Split infix into quotient and remainder
    fn split_infix(infix: u64, remainder_size: u8) -> (u64, u64) {
        let quotient = infix >> remainder_size;
        let remainder = infix & ((1 << remainder_size) - 1);
        (quotient, remainder)
    }

    /// Write a remainder value to a specific slot
    fn write_slot(slots_slice: &mut [u64], slot_index: usize, remainder: u64, remainder_size: u8) {
        let bit_pos = slot_index * remainder_size as usize;
        let word_index = bit_pos / U64_BITS;
        let bit_offset = bit_pos % U64_BITS;

        // clear the bits first
        let mask = ((1u64 << remainder_size) - 1) << bit_offset;
        slots_slice[word_index] &= !mask;

        // write the remainder
        slots_slice[word_index] |= (remainder & ((1u64 << remainder_size) - 1)) << bit_offset;

        // handle overflow to next word if needed
        if bit_offset + remainder_size as usize > U64_BITS {
            let overflow_bits = (bit_offset + remainder_size as usize) - U64_BITS;
            let overflow_mask = (1u64 << overflow_bits) - 1;
            slots_slice[word_index + 1] &= !overflow_mask;
            slots_slice[word_index + 1] |= remainder >> (remainder_size as usize - overflow_bits);
        }
    }

    /// Compute and store popcounts for first half. Optimization for rank queries
    fn compute_popcounts(
        data: &mut [u64],
        occupieds_start: usize,
        runends_start: usize,
        num_slots: u16,
    ) {
        let occupieds_half = TARGET_SIZE as usize / 2;
        let runends_half = num_slots as usize / 2;

        let occupieds_words = (TARGET_SIZE as usize + U64_BITS - 1) / U64_BITS;
        let runends_words = (num_slots as usize + U64_BITS - 1) / U64_BITS;

        let occupieds_slice = &data[occupieds_start..occupieds_start + occupieds_words];
        let runends_slice = &data[runends_start..runends_start + runends_words];

        let occupieds_popcount = rank(occupieds_slice, occupieds_half) as u32;
        let runends_popcount = rank(runends_slice, runends_half) as u32;

        // store in first word: [occupieds_popcount: 32 bits][runends_popcount: 32 bits]
        data[0] = ((occupieds_popcount as u64) << 32) | (runends_popcount as u64);
    }

    /// get memory layout offsets
    fn get_offsets(&self) -> (usize, usize, usize) {
        let num_slots = SCALED_SIZES[self.size_grade as usize];
        let occupieds_start = 1;
        let occupieds_words = (TARGET_SIZE as usize + U64_BITS - 1) / U64_BITS;
        let runends_start = occupieds_start + occupieds_words;
        let runends_words = (num_slots as usize + U64_BITS - 1) / U64_BITS;
        let slots_start = runends_start + runends_words;

        (occupieds_start, runends_start, slots_start)
    }

    /// check if a quotient bit is set in occupieds
    pub fn is_occupied(&self, quotient: usize) -> bool {
        let (occupieds_start, _, _) = self.get_offsets();
        let occupieds_words = (TARGET_SIZE as usize + U64_BITS - 1) / U64_BITS;
        let occupieds_slice = &self.data[occupieds_start..occupieds_start + occupieds_words];
        get_bit(occupieds_slice, quotient)
    }

    /// check if a slot position has runend bit set
    pub fn is_runend(&self, slot_pos: usize) -> bool {
        let num_slots = SCALED_SIZES[self.size_grade as usize];
        let (_, runends_start, _) = self.get_offsets();
        let runends_words = (num_slots as usize + U64_BITS - 1) / U64_BITS;
        let runends_slice = &self.data[runends_start..runends_start + runends_words];
        get_bit(runends_slice, slot_pos)
    }

    /// read remainder value from a specific slot
    pub fn read_slot(&self, slot_index: usize) -> u64 {
        let num_slots = SCALED_SIZES[self.size_grade as usize];
        let (_, _, slots_start) = self.get_offsets();
        let slots_words =
            (num_slots as usize * self.remainder_size as usize + U64_BITS - 1) / U64_BITS;
        let slots_slice = &self.data[slots_start..slots_start + slots_words];

        let bit_pos = slot_index * self.remainder_size as usize;
        let word_index = bit_pos / U64_BITS;
        let bit_offset = bit_pos % U64_BITS;

        let mut result =
            (slots_slice[word_index] >> bit_offset) & ((1u64 << self.remainder_size) - 1);

        // handle overflow from next word if needed
        if bit_offset + self.remainder_size as usize > U64_BITS {
            let overflow_bits = (bit_offset + self.remainder_size as usize) - U64_BITS;
            let overflow_mask = (1u64 << overflow_bits) - 1;
            let overflow_value = slots_slice[word_index + 1] & overflow_mask;
            result |= overflow_value << (self.remainder_size as usize - overflow_bits);
        }

        result
    }

    pub fn elem_count(&self) -> usize {
        self.elem_count as usize
    }

    pub fn size_grade(&self) -> u8 {
        self.size_grade
    }

    pub fn remainder_size(&self) -> u8 {
        self.remainder_size
    }

    pub fn num_slots(&self) -> usize {
        SCALED_SIZES[self.size_grade as usize] as usize
    }

    pub fn pretty_print(&self) {
        print!("{}", self);
    }
}

impl fmt::Display for InfixStore {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let num_slots = SCALED_SIZES[self.size_grade as usize];

        writeln!(f, "*** InfixStore ***")?;
        writeln!(f, "elem_count: {}", self.elem_count)?;
        writeln!(f, "size_grade: {}", self.size_grade)?;
        writeln!(f, "num_slots: {}", num_slots)?;
        writeln!(f, "remainder_size: {} bits", self.remainder_size)?;
        writeln!(f)?;

        writeln!(f, "popcounts: 0x{:016x}", self.data[0])?;
        let occupieds_popcount = (self.data[0] & 0xFFFFFFFF) as u32;
        let runends_popcount = (self.data[0] >> 32) as u32;
        writeln!(f, "  occupieds_popcount: {}", occupieds_popcount)?;
        writeln!(f, "  runends_popcount: {}", runends_popcount)?;
        writeln!(f)?;

        writeln!(f, "occupieds bitmap (showing set quotients):")?;
        let mut occupied_quotients = Vec::new();
        for q in 0..TARGET_SIZE as usize {
            if self.is_occupied(q) {
                occupied_quotients.push(q);
            }
        }
        if occupied_quotients.is_empty() {
            writeln!(f, "  (none)")?;
        } else {
            for chunk in occupied_quotients.chunks(16) {
                write!(f, "  ")?;
                for &q in chunk {
                    write!(f, "{} ", q)?;
                }
                writeln!(f)?;
            }
        }
        writeln!(f)?;

        writeln!(f, "runends bitmap (showing runend positions):")?;
        let mut runend_positions = Vec::new();
        for pos in 0..num_slots as usize {
            if self.is_runend(pos) {
                runend_positions.push(pos);
            }
        }
        if runend_positions.is_empty() {
            writeln!(f, "  (none)")?;
        } else {
            for chunk in runend_positions.chunks(16) {
                write!(f, "  ")?;
                for &pos in chunk {
                    write!(f, "{} ", pos)?;
                }
                writeln!(f)?;
            }
        }
        writeln!(f)?;

        writeln!(f, "slots (pos: value [runend]):")?;
        if self.elem_count == 0 {
            writeln!(f, "  (empty)")?;
        } else {
            for i in 0..self.elem_count as usize {
                let value = self.read_slot(i);
                let is_runend = self.is_runend(i);
                writeln!(
                    f,
                    "  {}: {} {}",
                    i,
                    value,
                    if is_runend { "[R]" } else { "" }
                )?;
            }
        }
        writeln!(f, "*************************")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_infix() {
        // infix = 0b1010101010101010 (16 bits)
        // remainder_size = 8
        let infix = 0b1010101010101010u64;
        let (quotient, remainder) = InfixStore::split_infix(infix, 8);

        assert_eq!(quotient, 0b10101010); // top 8 bits
        assert_eq!(remainder, 0b10101010); // bottom 8 bits

        // test with different sizes
        let infix = 0b11110000_11001100u64;
        let (quotient, remainder) = InfixStore::split_infix(infix, 8);
        assert_eq!(quotient, 0b11110000);
        assert_eq!(remainder, 0b11001100);
    }

    #[test]
    fn test_construction_simple() {
        // infixes with 10 bits quotient and 8 bits remainder
        // quotient|remainder format
        let infixes = vec![
            (129u64 << 8) | 170,
            (129u64 << 8) | 188,
            (129u64 << 8) | 207,
            (340u64 << 8) | 51,
            (340u64 << 8) | 90,
        ];

        let store = InfixStore::new_with_infixes(&infixes, 8);

        assert_eq!(store.elem_count, 5);
        // assert_eq!(store.quotient_size, 10);
        assert_eq!(store.remainder_size, 8);

        // verify occupieds: quotients 129 and 340 should be set
        assert!(store.is_occupied(129));
        assert!(store.is_occupied(340));
        assert!(!store.is_occupied(0));
        assert!(!store.is_occupied(200));

        // verify runends: slots 2 and 4 should be marked (end of each run)
        assert!(!store.is_runend(0));
        assert!(!store.is_runend(1));
        assert!(store.is_runend(2)); // end of q=129's run
        assert!(!store.is_runend(3));
        assert!(store.is_runend(4)); // end of q=340's run

        // verify remainders in slots
        assert_eq!(store.read_slot(0), 170);
        assert_eq!(store.read_slot(1), 188);
        assert_eq!(store.read_slot(2), 207);
        assert_eq!(store.read_slot(3), 51);
        assert_eq!(store.read_slot(4), 90);
    }

    #[test]
    fn test_construction_same_quotient() {
        // all elements have same quotient
        let infixes = vec![
            (50u64 << 8) | 10,
            (50u64 << 8) | 20,
            (50u64 << 8) | 30,
            (50u64 << 8) | 40,
        ];

        let store = InfixStore::new_with_infixes(&infixes, 8);

        assert_eq!(store.elem_count, 4);
        assert!(store.is_occupied(50));
        assert!(!store.is_occupied(49));
        assert!(!store.is_occupied(51));

        // all in same run, only last slot is runend
        assert!(!store.is_runend(0));
        assert!(!store.is_runend(1));
        assert!(!store.is_runend(2));
        assert!(store.is_runend(3));

        assert_eq!(store.read_slot(0), 10);
        assert_eq!(store.read_slot(1), 20);
        assert_eq!(store.read_slot(2), 30);
        assert_eq!(store.read_slot(3), 40);
    }

    #[test]
    fn test_construction_different_quotients() {
        // each element has different quotient
        let infixes = vec![(10u64 << 8) | 100, (20u64 << 8) | 101, (30u64 << 8) | 102];

        let store = InfixStore::new_with_infixes(&infixes, 8);

        assert_eq!(store.elem_count, 3);

        // all quotients occupied
        assert!(store.is_occupied(10));
        assert!(store.is_occupied(20));
        assert!(store.is_occupied(30));

        // each slot is end of its own run
        assert!(store.is_runend(0));
        assert!(store.is_runend(1));
        assert!(store.is_runend(2));

        assert_eq!(store.read_slot(0), 100);
        assert_eq!(store.read_slot(1), 101);
        assert_eq!(store.read_slot(2), 102);
    }

    #[test]
    fn test_empty_store() {
        let infixes: Vec<u64> = vec![];
        let store = InfixStore::new_with_infixes(&infixes, 8);

        assert_eq!(store.elem_count, 0);
    }

    #[test]
    fn test_remainder_size_variations() {
        // test with different remainder sizes
        for remainder_size in [4, 6, 8, 10, 12] {
            let max_remainder = (1u64 << remainder_size) - 1;
            let infixes = vec![
                (100u64 << remainder_size) | max_remainder,
                (100u64 << remainder_size) | (max_remainder - 1),
            ];

            let store = InfixStore::new_with_infixes(&infixes, remainder_size);

            assert_eq!(store.remainder_size, remainder_size);
            assert_eq!(store.read_slot(0), max_remainder);
            assert_eq!(store.read_slot(1), max_remainder - 1);
        }
    }
}
