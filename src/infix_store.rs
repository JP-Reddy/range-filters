use crate::U64_BITS;
use crate::bitmap::{clear_bit, get_bit, rank, select, set_bit};
use std::fmt;

const TARGET_SIZE: u16 = 1024;
const QUOTIENT_SIZE: u8 = 10;
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
    quotient_size: u8,
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
        // let quotient_size = (TARGET_SIZE as usize + U64_BITS - 1) / U64_BITS - 1;
        // println!("quotient_size: {}", quotient_size);

        if infixes.is_empty() {
            return Self {
                elem_count: 0,
                size_grade,
                remainder_size,
                quotient_size: QUOTIENT_SIZE,
                data,
            };
        }

        // step 3: load infixes in the infix store
        Self::load_infixes_to_store(&mut data, infixes, QUOTIENT_SIZE, remainder_size, num_slots);

        Self {
            elem_count: infixes.len() as u16,
            size_grade,
            remainder_size,
            quotient_size: QUOTIENT_SIZE,
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
        quotient_size: u8,
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
            let (quotient, remainder) = Self::split_infix(*infix, quotient_size, remainder_size);

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
    fn split_infix(infix: u64, quotient_size: u8, remainder_size: u8) -> (u64, u64) {
        // extract remainder (bottom remainder_size bits)
        let remainder = infix & ((1 << remainder_size) - 1);

        let quotient = (infix >> remainder_size) & ((1 << (quotient_size)) - 1);
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

    /// insert a key into the infix store
    pub fn insert(&mut self, infix: u64) -> bool {
        let mut num_slots = SCALED_SIZES[self.size_grade as usize];

        // check if we have enough space and resize if possible
        if self.elem_count >= num_slots {
            if !self.resize_up() {
                return false;
            }
            num_slots = SCALED_SIZES[self.size_grade as usize];
        }

        let (quotient, remainder) = Self::split_infix(infix, self.quotient_size, self.remainder_size);
        let (occupieds_start, runends_start, slots_start) = self.get_offsets();
        let occupieds_words = (TARGET_SIZE as usize + U64_BITS - 1) / U64_BITS;
        let runends_words = (num_slots as usize + U64_BITS - 1) / U64_BITS;
        let slots_words =
            (num_slots as usize * self.remainder_size as usize + U64_BITS - 1) / U64_BITS;

        // check if this quotient already has a run
        let is_new_quotient = !self.is_occupied(quotient as usize);

        // find position to insert new remainder
        let occupieds_slice = &self.data[occupieds_start..occupieds_start + occupieds_words];
        let runends_slice = &self.data[runends_start..runends_start + runends_words];
        let run_index = rank(occupieds_slice, quotient as usize);
        let run_start = if run_index == 0 {
            0
        } else {
            select(runends_slice, run_index - 1)
                .map(|x| x + 1)
                .unwrap_or(0)
        };
        let run_end = select(runends_slice, run_index).unwrap_or(self.elem_count as usize);

        let insert_pos;
        if is_new_quotient {
            insert_pos = run_start;
        } else {
            // assume insertion position at the end to begin with
            // if a run with a greater remainder is found, update position
            let mut found_pos = run_end + 1;
            for i in run_start..=run_end {
                let val = self.read_slot(i);
                // the key already exists
                if val == remainder {
                    return true;
                }

                if val > remainder {
                    found_pos = i;
                    break;
                }
            }
            insert_pos = found_pos;
        }

        // shift the slots and runends to make room
        if self.elem_count > 0 {
            self.shift_slots_right(insert_pos);
            self.shift_runends_right(insert_pos);
        }

        // insert the remainder in the new empty slot
        let slots_slice = &mut self.data[slots_start..slots_start + slots_words];
        Self::write_slot(slots_slice, insert_pos, remainder, self.remainder_size);

        let runends_slice = &mut self.data[runends_start..runends_start + runends_words];
        // set both runend and occupieds bits if new quotient
        if is_new_quotient {
            set_bit(runends_slice, insert_pos);
            let occupieds_slice =
                &mut self.data[occupieds_start..occupieds_start + occupieds_words];
            set_bit(occupieds_slice, quotient as usize);
        } else {
            // if inserted after the old run_end, clear and set new run_end
            if insert_pos >= run_end {
                clear_bit(runends_slice, run_end);
                set_bit(runends_slice, insert_pos);
            }
        }
        // increment element count
        self.elem_count += 1;
        true
    }

    /// delete a key from the infix store
    pub fn delete(&mut self, infix: u64) -> bool {
        // check if the quotient exists
        let (quotient, remainder) = Self::split_infix(infix, self.quotient_size, self.remainder_size);
        if !self.is_occupied(quotient as usize) {
            return false;
        }

        let num_slots = SCALED_SIZES[self.size_grade as usize];
        let (occupieds_start, runends_start, _) = self.get_offsets();
        let occupieds_words = (TARGET_SIZE as usize + U64_BITS - 1) / U64_BITS;
        let runends_words = (num_slots as usize + U64_BITS - 1) / U64_BITS;

        // find the run index
        let occupieds_slice = &self.data[occupieds_start..occupieds_start + occupieds_words];
        let runends_slice = &self.data[runends_start..runends_start + runends_words];
        let run_index = rank(occupieds_slice, quotient as usize);

        // calculate run start and end
        let run_start = if run_index == 0 {
            0
        } else {
            select(runends_slice, run_index - 1)
                .map(|x| x + 1)
                .unwrap_or(0)
        };
        let run_end =
            select(runends_slice, run_index).expect("panic: occupied bit set but no runend found.");

        // find the slot position to be deleted
        let mut del_pos = None;
        for i in run_start..=run_end {
            if self.read_slot(i) == remainder {
                del_pos = Some(i);
                break;
            }
        }
        let pos = match del_pos {
            Some(p) => p,
            None => return false,
        };

        let runends_slice = &mut self.data[runends_start..runends_start + runends_words];

        let is_last_item_in_run = run_start == run_end;
        if is_last_item_in_run {
            // if only item remaining in the run, remove the quotient as well
            let occupieds_slice =
                &mut self.data[occupieds_start..occupieds_start + occupieds_words];
            clear_bit(occupieds_slice, quotient as usize);
        } else if pos == run_end {
            // if last item of a multi-item run, mark previous item as the new run_end
            set_bit(runends_slice, pos - 1);
        }

        // shift slots and runends to the left and delete the remainder
        self.shift_slots_left(pos);
        self.shift_runends_left(pos);

        // decrement element count
        self.elem_count -= 1;

        // check if we can size down a grade
        if self.size_grade > 0 {
            let prev_size_grade = SCALED_SIZES[(self.size_grade - 1) as usize];
            if self.elem_count <= prev_size_grade / 2 {
                self.resize_down();
            }
        }

        true
    }

    fn resize_to(&mut self, new_size_grade: u8) {
        let new_num_slots = SCALED_SIZES[new_size_grade as usize];

        // initialize new data vector
        let popcounts_words = 1;
        let occupieds_words = (TARGET_SIZE as usize + U64_BITS - 1) / U64_BITS;
        let new_runends_words = (new_num_slots as usize + U64_BITS - 1) / U64_BITS;
        let new_slots_words =
            (new_num_slots as usize * self.remainder_size as usize + U64_BITS - 1) / U64_BITS;
        let total_words = popcounts_words + occupieds_words + new_runends_words + new_slots_words;
        let mut new_data = vec![0u64; total_words];

        // calculate new offsets
        let occupieds_start = popcounts_words;
        let runends_start = occupieds_start + occupieds_words;
        let new_slots_start = runends_start + new_runends_words;

        // old offsets to read from
        let old_num_slots = SCALED_SIZES[self.size_grade as usize];
        let old_runends_words = (old_num_slots as usize + U64_BITS - 1) / U64_BITS;
        let old_slots_start = runends_start + old_runends_words;

        // copy popcounts and occupied (fixed data)
        let fixed_region_len = runends_start;
        new_data[0..fixed_region_len].copy_from_slice(&self.data[0..fixed_region_len]);

        // copy runends containing data
        let valid_runends_words = (self.elem_count as usize + U64_BITS - 1) / U64_BITS;
        if valid_runends_words > 0 {
            new_data[runends_start..runends_start + valid_runends_words]
                .copy_from_slice(&self.data[runends_start..runends_start + valid_runends_words]);
        }

        // copy slots containing data
        let valid_slots_bits = self.elem_count as usize * self.remainder_size as usize;
        let valid_slots_words = (valid_slots_bits + U64_BITS - 1) / U64_BITS;
        if valid_slots_words > 0 {
            new_data[new_slots_start..new_slots_start + valid_slots_words]
                .copy_from_slice(&self.data[old_slots_start..old_slots_start + valid_slots_words]);
        }

        self.data = new_data;
        self.size_grade = new_size_grade;
    }

    fn resize_up(&mut self) -> bool {
        // fail if already at max size
        if self.size_grade as usize >= SIZE_GRADE_COUNT - 1 {
            return false;
        }
        self.resize_to(self.size_grade + 1);
        true
    }

    fn resize_down(&mut self) -> bool {
        // fail if already at the min size
        if self.size_grade == 0 {
            return false;
        }
        self.resize_to(self.size_grade - 1);
        true
    }

    /// shift all slots from start_pos to the right by 1 (for insertion)
    fn shift_slots_right(&mut self, start_pos: usize) {
        let num_slots = SCALED_SIZES[self.size_grade as usize];
        let (_, _, slots_start) = self.get_offsets();
        let slots_words =
            (num_slots as usize * self.remainder_size as usize + U64_BITS - 1) / U64_BITS;

        for i in (start_pos..self.elem_count as usize).rev() {
            let value = self.read_slot(i);
            let slots_slice = &mut self.data[slots_start..slots_start + slots_words];
            Self::write_slot(slots_slice, i + 1, value, self.remainder_size);
        }
    }

    /// shift all runend bits from start_pos to the right by 1 (for insertion)
    fn shift_runends_right(&mut self, start_pos: usize) {
        let num_slots = SCALED_SIZES[self.size_grade as usize];
        let (_, runends_start, _) = self.get_offsets();
        let runends_words = (num_slots as usize + U64_BITS - 1) / U64_BITS;
        let runends_slice = &mut self.data[runends_start..runends_start + runends_words];

        for i in (start_pos..self.elem_count as usize).rev() {
            let bit_value = get_bit(runends_slice, i);
            if bit_value {
                set_bit(runends_slice, i + 1);
            } else {
                clear_bit(runends_slice, i + 1);
            }
        }
        clear_bit(runends_slice, start_pos);
    }

    /// shift all slots to the left by 1 (after deletion)
    fn shift_slots_left(&mut self, start_pos: usize) {
        let num_slots = SCALED_SIZES[self.size_grade as usize];
        let (_, _, slots_start) = self.get_offsets();
        let slots_words =
            (num_slots as usize * self.remainder_size as usize + U64_BITS - 1) / U64_BITS;

        for i in start_pos..(self.elem_count as usize - 1) {
            let value = self.read_slot(i + 1);
            let slots_slice = &mut self.data[slots_start..slots_start + slots_words];
            Self::write_slot(slots_slice, i, value, self.remainder_size);
        }
    }

    /// shift all runend bits to the left by 1 (after deletion)
    fn shift_runends_left(&mut self, start_pos: usize) {
        let num_slots = SCALED_SIZES[self.size_grade as usize];
        let (_, runends_start, _) = self.get_offsets();
        let runends_words = (num_slots as usize + U64_BITS - 1) / U64_BITS;
        let runends_slice = &mut self.data[runends_start..runends_start + runends_words];

        for i in start_pos..(self.elem_count as usize - 1) {
            let bit_value = get_bit(runends_slice, i + 1);
            if bit_value {
                set_bit(runends_slice, i);
            } else {
                clear_bit(runends_slice, i);
            }
        }
        clear_bit(runends_slice, self.elem_count as usize - 1);
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

    /// Function to convert a key to infix using consistent extraction logic
    /// Returns the infix value for the given key within the predecessor/successor range
    fn convert_key_to_infix(
        &self,
        key: u64,
        predecessor_key: u64,
        successor_key: u64,
        remainder_size: u8,
    ) -> u64 {
        use crate::diva::Diva;

        let (shared_prefix_len, redundant_bits, quotient_bits) =
            Diva::get_shared_ignore_implicit_size(&predecessor_key, &successor_key, false);

        Diva::extract_partial_key(
            key,
            shared_prefix_len,
            redundant_bits,
            quotient_bits,
            remainder_size,
        )
    }

    /// Point query: check if a key exists in this InfixStore
    ///
    /// # Arguments
    /// * `query_key` - The key to search for
    /// * `predecessor_key` - The predecessor sample key
    /// * `successor_key` - The successor sample key
    /// * `remainder_size` - Number of bits in remainder
    pub fn point_query(
        &self,
        query_key: u64,
        predecessor_key: u64,
        successor_key: u64,
        remainder_size: u8,
    ) -> bool {
        let infix =
            self.convert_key_to_infix(query_key, predecessor_key, successor_key, remainder_size);

        let (quotient, remainder) = Self::split_infix(infix, self.quotient_size, self.remainder_size);

        // Check if quotient exists in occupieds bitmap
        if !self.is_occupied(quotient as usize) {
            return false;
        }

        // Find the run for this quotient and scan for exact remainder match
        let result = self.find_remainder_in_run(quotient as usize, remainder);
        result
    }

    /// Find a specific remainder value within a quotient's run
    fn find_remainder_in_run(&self, quotient: usize, target_remainder: u64) -> bool {
        let (run_start, run_end) = match self.get_run_bounds(quotient) {
            Some(bounds) => bounds,
            None => {
                return false;
            }
        };

        for pos in run_start..=run_end {
            let remainder = self.read_slot(pos);
            if remainder == target_remainder {
                return true;
            }
        }

        false
    }

    /// Range query: check if any key exists in the given range [start_key, end_key] (inclusive)
    ///
    /// # Arguments
    /// * `start_key` - Start of the query range
    /// * `end_key` - End of the query range
    /// * `predecessor_key` - The predecessor sample key
    /// * `successor_key` - The successor sample key
    /// * `remainder_size` - Number of bits in remainder
    pub fn range_query(
        &self,
        start_key: u64,
        end_key: u64,
        predecessor_key: u64,
        successor_key: u64,
        remainder_size: u8,
    ) -> bool {
        if start_key > end_key {
            return false;
        }

        // Convert both endpoints to infixes
        let start_infix =
            self.convert_key_to_infix(start_key, predecessor_key, successor_key, remainder_size);
        let end_infix =
            self.convert_key_to_infix(end_key, predecessor_key, successor_key, remainder_size);

        // Split infixes into quotients and remainders
        let (start_quotient, start_remainder) = Self::split_infix(start_infix, self.quotient_size, remainder_size);
        let (end_quotient, end_remainder) = Self::split_infix(end_infix, self.quotient_size, remainder_size);

        // Handle the two main cases
        if start_quotient == end_quotient {
            // Case 1: Range spans single quotient
            self.query_single_quotient_range(
                start_quotient as usize,
                start_remainder,
                end_remainder,
            )
        } else {
            // Case 2: Range spans multiple quotients
            self.query_multiple_quotient_range(
                start_quotient as usize,
                start_remainder,
                end_quotient as usize,
                end_remainder,
            )
        }
    }

    /// Handle range query when both endpoints map to the same quotient
    fn query_single_quotient_range(
        &self,
        quotient: usize,
        start_remainder: u64,
        end_remainder: u64,
    ) -> bool {
        // Check if quotient exists
        if !self.is_occupied(quotient) {
            println!("    Quotient {} not occupied", quotient);
            return false;
        }

        // Find the run and scan for any remainder in the range [start_remainder, end_remainder]
        self.scan_run_for_range(quotient, start_remainder, end_remainder)
    }

    /// Handle range query when endpoints map to different quotients
    fn query_multiple_quotient_range(
        &self,
        start_quotient: usize,
        start_remainder: u64,
        end_quotient: usize,
        end_remainder: u64,
    ) -> bool {
        // Check for any occupied quotients strictly between start_quotient and end_quotient
        if start_quotient + 1 < end_quotient {
            let (occupieds_start, _, _) = self.get_offsets();
            let occupieds_words = (TARGET_SIZE as usize + crate::U64_BITS - 1) / crate::U64_BITS;
            let occupieds_slice = &self.data[occupieds_start..occupieds_start + occupieds_words];

            if crate::has_bits_in_range(occupieds_slice, start_quotient + 1, end_quotient) {
                return true; // All remainders in intermediate quotients are within range
            }
        }

        // Check start quotient for remainders >= start_remainder
        if self.is_occupied(start_quotient) {
            if self.scan_run_from_remainder(start_quotient, start_remainder, true) {
                return true;
            }
        }

        // Check end quotient for remainders <= end_remainder
        if self.is_occupied(end_quotient) {
            if self.scan_run_from_remainder(end_quotient, end_remainder, false) {
                return true;
            }
        }

        false
    }

    /// Scan a quotient's run for any remainder in the range [start_remainder, end_remainder]
    fn scan_run_for_range(
        &self,
        quotient: usize,
        start_remainder: u64,
        end_remainder: u64,
    ) -> bool {
        let (run_start, run_end) = match self.get_run_bounds(quotient) {
            Some(bounds) => bounds,
            None => return false,
        };

        let mut pos = run_end;
        loop {
            let remainder = self.read_slot(pos);

            // Check if remainder is in our target range
            if remainder >= start_remainder && remainder <= end_remainder {
                return true;
            }

            if remainder < start_remainder {
                return false;
            }

            // Continue to previous position if we haven't reached the start
            if pos == run_start {
                break;
            }
            pos -= 1;
        }

        false
    }

    /// Scan a quotient's run for remainders >= threshold (if ascending) or <= threshold (if !ascending)
    /// Uses optimized scanning direction based on query type
    fn scan_run_from_remainder(
        &self,
        quotient: usize,
        threshold_remainder: u64,
        ascending: bool,
    ) -> bool {
        let (run_start, run_end) = match self.get_run_bounds(quotient) {
            Some(bounds) => bounds,
            None => return false,
        };

        if ascending {
            // For ascending queries (>= threshold), scan left-to-right
            for pos in run_start..=run_end {
                let remainder = self.read_slot(pos);

                if remainder >= threshold_remainder {
                    return true;
                }
            }
        } else {
            // For descending queries (<= threshold), scan right-to-left
            let mut pos = run_end;
            loop {
                let remainder = self.read_slot(pos);

                if remainder <= threshold_remainder {
                    return true;
                }

                // Continue to previous position if we haven't reached the start
                if pos == run_start {
                    break;
                }
                pos -= 1;
            }
        }

        false
    }

    /// Get the start and end positions of a quotient's run
    fn get_run_bounds(&self, quotient: usize) -> Option<(usize, usize)> {
        // Get the rank of this quotient (how many quotients before it)
        let (occupieds_start, runends_start, _) = self.get_offsets();
        let occupieds_words = (TARGET_SIZE as usize + U64_BITS - 1) / U64_BITS;
        let occupieds_slice = &self.data[occupieds_start..occupieds_start + occupieds_words];

        let rank_result = rank(occupieds_slice, quotient);

        // Use select to find the end position of this run
        let num_slots = SCALED_SIZES[self.size_grade as usize];
        let runends_words = (num_slots as usize + U64_BITS - 1) / U64_BITS;
        let runends_slice = &self.data[runends_start..runends_start + runends_words];

        let run_end = match select(runends_slice, rank_result) {
            Some(pos) => pos,
            None => return None,
        };

        // Find the start of this run by looking backwards
        let mut run_start = run_end;
        while run_start > 0 && !self.is_runend(run_start - 1) {
            run_start -= 1;
        }

        Some((run_start, run_end))
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
        let (quotient, remainder) = InfixStore::split_infix(infix, 10, 8);

        assert_eq!(quotient, 0b10101010); // top 8 bits
        assert_eq!(remainder, 0b10101010); // bottom 8 bits

        // test with different sizes
        let infix = 0b11110000_11001100u64;
        let (quotient, remainder) = InfixStore::split_infix(infix, 10, 8);
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

    #[test]
    fn test_insert_middle_of_run() {
        let infixes = vec![(100u64 << 8) | 10, (100u64 << 8) | 30];
        let mut store = InfixStore::new_with_infixes(&infixes, 8);

        store.insert((100u64 << 8) | 20);

        assert_eq!(store.elem_count, 3);
        assert_eq!(store.read_slot(0), 10);
        assert_eq!(store.read_slot(1), 20);
        assert_eq!(store.read_slot(2), 30);
        assert!(store.is_runend(2));
    }

    #[test]
    fn test_insert_end_of_run() {
        let infixes = vec![(100u64 << 8) | 10, (100u64 << 8) | 20];
        let mut store = InfixStore::new_with_infixes(&infixes, 8);

        store.insert((100u64 << 8) | 30);

        assert_eq!(store.elem_count, 3);
        assert_eq!(store.read_slot(0), 10);
        assert_eq!(store.read_slot(1), 20);
        assert_eq!(store.read_slot(2), 30);
        assert!(store.is_runend(2));
        assert!(!store.is_runend(1));
    }

    #[test]
    fn test_insert_beginning_of_run() {
        let infixes = vec![(100u64 << 8) | 20, (100u64 << 8) | 30];
        let mut store = InfixStore::new_with_infixes(&infixes, 8);

        store.insert((100u64 << 8) | 10);

        assert_eq!(store.elem_count, 3);
        assert_eq!(store.read_slot(0), 10);
        assert_eq!(store.read_slot(1), 20);
        assert_eq!(store.read_slot(2), 30);
        assert!(store.is_runend(2));
    }

    #[test]
    fn test_insert_new_quotient() {
        let infixes = vec![(100u64 << 8) | 10, (200u64 << 8) | 20];
        let mut store = InfixStore::new_with_infixes(&infixes, 8);

        store.insert((150u64 << 8) | 15);

        assert_eq!(store.elem_count, 3);
        assert!(store.is_occupied(100));
        assert!(store.is_occupied(150));
        assert!(store.is_occupied(200));
        assert!(store.is_runend(0));
        assert!(store.is_runend(1));
        assert!(store.is_runend(2));
    }

    #[test]
    fn test_insert_duplicates() {
        let infixes = vec![(100u64 << 8) | 10];
        let mut store = InfixStore::new_with_infixes(&infixes, 8);

        assert!(store.insert((100u64 << 8) | 10));
        assert!(store.insert((100u64 << 8) | 10));

        assert_eq!(store.elem_count, 1);
        assert_eq!(store.read_slot(0), 10);
    }

    #[test]
    fn test_insert_boundary_values() {
        let infixes: Vec<u64> = vec![];
        let mut store = InfixStore::new_with_infixes(&infixes, 8);

        store.insert((0u64 << 8) | 0);
        store.insert((1023u64 << 8) | 255);
        store.insert((0u64 << 8) | 255);
        store.insert((1023u64 << 8) | 0);

        assert_eq!(store.elem_count, 4);
        assert!(store.is_occupied(0));
        assert!(store.is_occupied(1023));
    }

    #[test]
    fn test_insert_with_resize_up() {
        let infixes: Vec<u64> = vec![];
        let mut store = InfixStore::new_with_infixes(&infixes, 8);

        let initial_size_grade = store.size_grade();

        for i in 0..500 {
            store.insert((100u64 << 8) | i);
        }

        assert_eq!(store.elem_count, 500);
        assert!(store.size_grade() > initial_size_grade);
    }

    #[test]
    fn test_delete_middle_of_run() {
        let infixes = vec![(100u64 << 8) | 10, (100u64 << 8) | 20, (100u64 << 8) | 30];
        let mut store = InfixStore::new_with_infixes(&infixes, 8);

        assert!(store.delete((100u64 << 8) | 20));

        assert_eq!(store.elem_count, 2);
        assert_eq!(store.read_slot(0), 10);
        assert_eq!(store.read_slot(1), 30);
        assert!(store.is_runend(1));
    }

    #[test]
    fn test_delete_end_of_run() {
        let infixes = vec![(100u64 << 8) | 10, (100u64 << 8) | 20, (100u64 << 8) | 30];
        let mut store = InfixStore::new_with_infixes(&infixes, 8);

        assert!(store.delete((100u64 << 8) | 30));

        assert_eq!(store.elem_count, 2);
        assert_eq!(store.read_slot(0), 10);
        assert_eq!(store.read_slot(1), 20);
        assert!(store.is_runend(1));
        assert!(!store.is_runend(0));
    }

    #[test]
    fn test_delete_beginning_of_run() {
        let infixes = vec![(100u64 << 8) | 10, (100u64 << 8) | 20, (100u64 << 8) | 30];
        let mut store = InfixStore::new_with_infixes(&infixes, 8);

        assert!(store.delete((100u64 << 8) | 10));

        assert_eq!(store.elem_count, 2);
        assert_eq!(store.read_slot(0), 20);
        assert_eq!(store.read_slot(1), 30);
        assert!(store.is_runend(1));
    }

    #[test]
    fn test_delete_last_in_run() {
        let infixes = vec![(100u64 << 8) | 10, (200u64 << 8) | 20];
        let mut store = InfixStore::new_with_infixes(&infixes, 8);

        assert!(store.delete((100u64 << 8) | 10));

        assert_eq!(store.elem_count, 1);
        assert!(!store.is_occupied(100));
        assert!(store.is_occupied(200));
        assert_eq!(store.read_slot(0), 20);
        assert!(store.is_runend(0));
    }

    #[test]
    fn test_delete_nonexistent() {
        let infixes = vec![(100u64 << 8) | 10, (100u64 << 8) | 20];
        let mut store = InfixStore::new_with_infixes(&infixes, 8);

        assert!(!store.delete((100u64 << 8) | 30));
        assert!(!store.delete((200u64 << 8) | 10));

        assert_eq!(store.elem_count, 2);
    }

    #[test]
    fn test_delete_with_resize_down() {
        let infixes: Vec<u64> = vec![];
        let mut store = InfixStore::new_with_infixes(&infixes, 10);

        for i in 0..600 {
            store.insert((100u64 << 10) | i);
        }

        let size_after_insert = store.size_grade();

        for i in 0..500 {
            store.delete((100u64 << 10) | i);
        }

        assert_eq!(store.elem_count, 100);
        assert!(store.size_grade() < size_after_insert);
    }
}
