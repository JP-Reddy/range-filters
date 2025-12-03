use range_filters::data_gen::generate_smooth_u16;
use range_filters::diva::Diva;

fn main() {
    // Generate 3000 large keys from smooth distribution
    let mut keys = generate_smooth_u16(Some(3000));
    keys.sort();
    let keys = keys.into_iter().map(|k| k as u64).collect::<Vec<_>>();

    // Build DIVA filter with target size 1024
    let target_size = 1024;
    let false_positive_rate = 0.01; // 1%
    let diva = Diva::new_with_keys(&keys, target_size, false_positive_rate);

    // === Point Query Examples ===
    println!("\n=== Point Query Examples ===");

    // Test queries for existing keys (samples from different positions)
    let test_keys = vec![
        keys[0],                  // First key
        keys[keys.len() / 4],     // Quarter point
        keys[keys.len() / 2],     // Middle key
        keys[3 * keys.len() / 4], // Three quarter point
        keys[keys.len() - 1],     // Last key
    ];

    println!("Testing existing keys:");
    for &key in &test_keys {
        let result = diva.contains(key);
        println!(
            "  Key {}: {} (expected: found)",
            key,
            if result { "✓ FOUND" } else { "✗ NOT FOUND" }
        );
    }

    // Test queries for non-existing keys
    let first_key = keys[0];
    let last_key = keys[keys.len() - 1];
    let non_existing_keys = vec![
        first_key.saturating_sub(100),  // Before first key
        first_key + 1,                  // Just after first (might not exist)
        (first_key + last_key) / 2 + 1, // Middle range (might not exist)
        last_key + 100,                 // After last key
    ];

    println!("\nTesting potentially non-existing keys:");
    for &key in &non_existing_keys {
        let result = diva.contains(key);
        let actually_exists = keys.contains(&key);
        println!(
            "  Key {}: {} (actual: {})",
            key,
            if result { "✓ FOUND" } else { "✗ NOT FOUND" },
            if actually_exists {
                "exists"
            } else {
                "does not exist"
            }
        );

        // Note false positives
        if result && !actually_exists {
            println!("    ^ FALSE POSITIVE detected!");
        }
    }

    // === Range Query Examples ===
    println!("\n=== Range Query Examples ===");

    let first_key = keys[0];
    let last_key = keys[keys.len() - 1];
    let mid_key = keys[keys.len() / 2];

    // Test various range scenarios
    let range_tests = vec![
        // Small ranges
        (first_key, first_key + 100), // Small range at start
        (mid_key - 50, mid_key + 50), // Small range around middle
        (last_key - 100, last_key),   // Small range at end
        // Medium ranges
        (first_key, mid_key),                             // First half
        (mid_key, last_key),                              // Second half
        (keys[keys.len() / 4], keys[3 * keys.len() / 4]), // Middle 50%
        // Large ranges
        (first_key, last_key), // Full range
        // Edge cases
        (first_key.saturating_sub(1000), first_key.saturating_sub(1)), // Before all keys
        (last_key + 1, last_key + 1000),                               // After all keys
        (mid_key, mid_key),                                            // Single key range
    ];

    for &(start, end) in &range_tests {
        let result = diva.range_query(start, end);

        // Count actual keys in range
        let actual_keys_in_range: Vec<&u64> =
            keys.iter().filter(|&&k| k >= start && k <= end).collect();

        let has_keys = !actual_keys_in_range.is_empty();

        println!(
            "  Range [{}, {}]: {} (actual: {} keys)",
            start,
            end,
            if result {
                "✓ HAS KEYS"
            } else {
                "✗ NO KEYS"
            },
            actual_keys_in_range.len()
        );

        // Note false positives/negatives
        if result && !has_keys {
            println!("    ^ FALSE POSITIVE detected!");
        } else if !result && has_keys {
            println!("    ^ FALSE NEGATIVE detected! (This should not happen)");
        }

        // Show some keys in range for verification (limit to 10)
        if has_keys && actual_keys_in_range.len() <= 10 {
            println!("    Keys in range: {:?}", actual_keys_in_range);
        } else if has_keys && actual_keys_in_range.len() > 10 {
            println!(
                "    Keys in range: {:?}...{} more",
                &actual_keys_in_range[..5],
                actual_keys_in_range.len() - 5
            );
        }
    }
}
