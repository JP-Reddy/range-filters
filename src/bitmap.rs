const U64_BIT_SIZE: usize = 64;

#[inline]
pub fn set_bit(data: &mut [u64], pos: usize) {
    if pos >= data.len() * U64_BIT_SIZE {
        println!("pos: {} is out of bounds", pos);
        println!("data.len(): {}", data.len());
    }
    data[pos / U64_BIT_SIZE] |= 1 << (pos % U64_BIT_SIZE);
}

#[inline]
pub fn clear_bit(data: &mut [u64], pos: usize) {
    data[pos / U64_BIT_SIZE] &= !(1 << (pos % U64_BIT_SIZE));
}

#[inline]
pub fn get_bit(data: &[u64], pos: usize) -> bool {
    data[pos / U64_BIT_SIZE] & (1 << (pos % U64_BIT_SIZE)) != 0
}

// count the number of 1s in the data up to the pos
#[inline]
pub fn rank(data: &[u64], pos: usize) -> usize {
    let word_index = pos / U64_BIT_SIZE;
    let bit_index = pos % U64_BIT_SIZE;

    let mut count = 0;
    for i in 0..word_index {
        count += data[i].count_ones() as usize;
    }

    if bit_index > 0 {
        let mask = (1 << bit_index) - 1;
        count += (data[word_index] & mask).count_ones() as usize;
    }

    count
}

// find the position of the rank-th 1 in the data
#[inline]
pub fn select(data: &[u64], rank: usize) -> Option<usize> {
    let mut count = 0;

    let target = rank + 1;

    for (word_index, &word) in data.iter().enumerate() {
        let ones_in_word = word.count_ones() as usize;

        if count + ones_in_word >= target {
            let remaining = target - count;
            let pos_in_word = select_in_word(word, remaining - 1)?;
            return Some(word_index * U64_BIT_SIZE + pos_in_word);
        }

        count += ones_in_word;
    }
    None
}

#[inline]
fn select_in_word(word: u64, rank: usize) -> Option<usize> {
    let mut count = 0;

    for i in 0..U64_BIT_SIZE {
        if word & (1 << i) != 0 {
            if count == rank {
                return Some(i);
            }
            count += 1;
        }
    }
    None
}

/// optimized rank using cached halfway popcount
/// if pos is in second half, start counting from cached midpoint
#[inline]
pub fn rank_cached(data: &[u64], pos: usize, half_pos: usize, cached_popcount: usize) -> usize {
    if pos <= half_pos {
        // query is in first half, use regular rank
        rank(data, pos)
    } else {
        // query is in second half, start from cached count
        let word_offset = half_pos / U64_BIT_SIZE;
        let remaining = rank(&data[word_offset..], pos - half_pos);
        cached_popcount + remaining
    }
}

/// optimized select using cached halfway popcount
/// if target rank is past cached count, start from midpoint
#[inline]
pub fn select_cached(
    data: &[u64],
    rank_val: usize,
    half_pos: usize,
    cached_popcount: usize,
) -> Option<usize> {
    if rank_val < cached_popcount {
        // target is in first half, use regular select
        select(data, rank_val)
    } else {
        // target is in second half, start from cached midpoint
        let remaining_rank = rank_val - cached_popcount;
        let word_offset = half_pos / U64_BIT_SIZE;

        // search in second half
        select(&data[word_offset..], remaining_rank).map(|pos| pos + half_pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get_bit() {
        let mut data = vec![0u64; 2];

        // set bits in first word
        set_bit(&mut data, 0);
        set_bit(&mut data, 5);
        set_bit(&mut data, 63);

        assert!(get_bit(&data, 0));
        assert!(!get_bit(&data, 1));
        assert!(get_bit(&data, 5));
        assert!(get_bit(&data, 63));

        // set bits in second word
        set_bit(&mut data, 64);
        set_bit(&mut data, 127);

        assert!(get_bit(&data, 64));
        assert!(get_bit(&data, 127));
        assert!(!get_bit(&data, 100));
    }

    #[test]
    fn test_rank() {
        let mut data = vec![0u64; 2];

        set_bit(&mut data, 0);
        set_bit(&mut data, 2);
        set_bit(&mut data, 4);
        set_bit(&mut data, 64);
        set_bit(&mut data, 65);
        set_bit(&mut data, 127);

        assert_eq!(rank(&data, 0), 0); // before bit 0
        assert_eq!(rank(&data, 1), 1); // after bit 0
        assert_eq!(rank(&data, 3), 2); // after bits 0, 2
        assert_eq!(rank(&data, 5), 3); // after bits 0, 2, 4
        assert_eq!(rank(&data, 64), 3); // before second word
        assert_eq!(rank(&data, 65), 4); // after bit 64
        assert_eq!(rank(&data, 128), 6); // all bits
    }

    #[test]
    fn test_select() {
        let mut data = vec![0u64; 2];

        set_bit(&mut data, 0);
        set_bit(&mut data, 2);
        set_bit(&mut data, 4);
        set_bit(&mut data, 64);
        set_bit(&mut data, 65);
        set_bit(&mut data, 127);

        assert_eq!(select(&data, 0), Some(0)); // 0th one at position 0
        assert_eq!(select(&data, 1), Some(2)); // 1st one at position 2
        assert_eq!(select(&data, 2), Some(4)); // 2nd one at position 4
        assert_eq!(select(&data, 3), Some(64)); // 3rd one at position 64
        assert_eq!(select(&data, 4), Some(65)); // 4th one at position 65
        assert_eq!(select(&data, 5), Some(127)); // 5th one at position 127
        assert_eq!(select(&data, 6), None); // no 6th one
    }

    #[test]
    fn test_select_in_word() {
        // word = 0b10101 (bits 0, 2, 4 set)
        let word = 0b10101u64;

        assert_eq!(select_in_word(word, 0), Some(0));
        assert_eq!(select_in_word(word, 1), Some(2));
        assert_eq!(select_in_word(word, 2), Some(4));
        assert_eq!(select_in_word(word, 3), None);
    }

    #[test]
    fn test_rank_select_consistency() {
        let mut data = vec![0u64; 4];

        // set various bits
        let positions = vec![1, 7, 15, 63, 64, 100, 200, 255];
        for &pos in &positions {
            set_bit(&mut data, pos);
        }

        // verify rank-select consistency
        for (rank_, &expected_pos) in positions.iter().enumerate() {
            assert_eq!(select(&data, rank_), Some(expected_pos));
            assert_eq!(rank(&data, expected_pos + 1), rank_ + 1usize);
        }
    }
}
