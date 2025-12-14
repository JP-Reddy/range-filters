use range_filters::GrafiteFilter;

fn main() {
    // Create a filter with some keys
    let keys = [1, 2, 3, 7, 8, 9, 15, 20];
    let epsilon = 0.01; // 1% false positive rate

    let filter = GrafiteFilter::new_with_keys(&keys, epsilon);

    println!("Created GrafiteFilter with {} keys", filter.num_keys());
    println!("False positive rate: {}", filter.fpr());
    println!();

    // Test point queries
    println!("Point queries:");
    println!("  Key 7 exists: {}", filter.point_query(7));
    println!("  Key 10 exists: {}", filter.point_query(10));
    println!();

    // Test range queries (inclusive)
    println!("Range queries:");
    println!("  Range [3, 5] has values: {}", filter.range_query(3, 5));
    println!("  Range [9, 16] has values: {}", filter.range_query(9, 16));
    println!("  Range [10, 14] has values: {}", filter.range_query(10, 14));
    println!("  Range [10, 15] has values: {}", filter.range_query(10, 15));
    println!("  Range [21, 30] has values: {}", filter.range_query(21, 30));
}
