use fastbloom::BloomFilter;

fn range_query(bloom: &BloomFilter, start: u64, end: u64) -> bool {
    for key in start..=end {
        let key_str = key.to_string();
        if bloom.contains(&key_str) {
            return true;
        }
    }
    false
}

fn main() {
    let mut bloom = BloomFilter::with_false_pos(0.01).expected_items(1000);

    let keys = vec![100u64, 500, 1000, 5000, 10000];
    bloom.insert_all(&keys.iter().map(|&num| num.to_string()).collect::<Vec<String>>());

    println!("Keys:");
    println!("{:?}", keys);

    println!("\nPoint queries:");
    println!("  '500': {}", bloom.contains("500"));
    println!("  '999': {}", bloom.contains("999"));

    println!("\nRange queries:");
    println!("  [0, 200]: {}", range_query(&bloom, 0, 200));
    println!("  [200, 400]: {}", range_query(&bloom, 200, 400));
    println!("  [400, 600]: {}", range_query(&bloom, 400, 600));
}
