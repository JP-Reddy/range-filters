use divan::{black_box, Bencher};
use range_filters::{
    bloom_filter::BloomFilter,
    data_gen::generate_smooth_u64,
    diva::Diva,
    Key,
};
use rand::Rng;

fn main() {
    divan::main();
}

const SIZES: &[usize] = &[10_000, 100_000, 1_000_000, 10_000_000];

// generate query ranges for benchmarking
fn generate_query_ranges(keys: &[Key], percent: f64, num_queries: usize) -> Vec<(Key, Key)> {
    let mut rng = rand::thread_rng();
    let mut ranges = Vec::with_capacity(num_queries);

    let min_key = *keys.first().unwrap();
    let max_key = *keys.last().unwrap();
    let key_range = max_key - min_key;
    let span = (key_range as f64 * percent) as u64;

    for _ in 0..num_queries {
        let start = min_key + rng.gen_range(0..key_range.saturating_sub(span));
        let end = start + span;
        ranges.push((start, end.min(max_key)));
    }

    ranges
}

// ============================================================================
// DIVA Benchmarks
// ============================================================================

#[divan::bench(args = SIZES)]
fn diva_construction(bencher: Bencher, size: usize) {
    let keys = generate_smooth_u64(Some(size));

    bencher.bench_local(|| {
        black_box(Diva::new_with_keys(
            black_box(&keys),
            black_box(1024),
            black_box(0.01),
        ))
    });
}

#[divan::bench(args = SIZES)]
fn diva_point_query(bencher: Bencher, size: usize) {
    let keys = generate_smooth_u64(Some(size));
    let diva = Diva::new_with_keys(&keys, 1024, 0.01);

    // generate query keys (mix of existing and non-existing)
    let mut rng = rand::thread_rng();
    let query_keys: Vec<Key> = (0..1000)
        .map(|i| {
            if i % 2 == 0 {
                keys[rng.gen_range(0..keys.len())]
            } else {
                let idx = rng.gen_range(0..keys.len().saturating_sub(1));
                (keys[idx] + keys[idx + 1]) / 2
            }
        })
        .collect();

    let mut query_idx = 0;
    bencher.bench_local(|| {
        let key = query_keys[query_idx % query_keys.len()];
        query_idx += 1;
        black_box(diva.contains(black_box(key)))
    });
}

#[divan::bench(args = SIZES)]
fn diva_range_query_small(bencher: Bencher, size: usize) {
    let keys = generate_smooth_u64(Some(size));
    let diva = Diva::new_with_keys(&keys, 1024, 0.01);
    let query_ranges = generate_query_ranges(&keys, 0.01, 1000);

    let mut query_idx = 0;
    bencher.bench_local(|| {
        let (start, end) = query_ranges[query_idx % query_ranges.len()];
        query_idx += 1;
        black_box(diva.range_query(black_box(start), black_box(end)))
    });
}

#[divan::bench(args = SIZES)]
fn diva_range_query_medium(bencher: Bencher, size: usize) {
    let keys = generate_smooth_u64(Some(size));
    let diva = Diva::new_with_keys(&keys, 1024, 0.01);
    let query_ranges = generate_query_ranges(&keys, 0.07, 1000);

    let mut query_idx = 0;
    bencher.bench_local(|| {
        let (start, end) = query_ranges[query_idx % query_ranges.len()];
        query_idx += 1;
        black_box(diva.range_query(black_box(start), black_box(end)))
    });
}

#[divan::bench(args = SIZES)]
fn diva_range_query_large(bencher: Bencher, size: usize) {
    let keys = generate_smooth_u64(Some(size));
    let diva = Diva::new_with_keys(&keys, 1024, 0.01);
    let query_ranges = generate_query_ranges(&keys, 0.4, 1000);

    let mut query_idx = 0;
    bencher.bench_local(|| {
        let (start, end) = query_ranges[query_idx % query_ranges.len()];
        query_idx += 1;
        black_box(diva.range_query(black_box(start), black_box(end)))
    });
}

// ============================================================================
// Bloom Filter Benchmarks
// ============================================================================

#[divan::bench(args = SIZES)]
fn bloom_construction(bencher: Bencher, size: usize) {
    let keys = generate_smooth_u64(Some(size));

    bencher.bench_local(|| {
        black_box(BloomFilter::new_with_keys(
            black_box(&keys),
            black_box(1024),
            black_box(0.01),
        ))
    });
}

#[divan::bench(args = SIZES)]
fn bloom_point_query(bencher: Bencher, size: usize) {
    let keys = generate_smooth_u64(Some(size));
    let bloom = BloomFilter::new_with_keys(&keys, 1024, 0.01);

    let mut rng = rand::thread_rng();
    let query_keys: Vec<Key> = (0..1000)
        .map(|i| {
            if i % 2 == 0 {
                keys[rng.gen_range(0..keys.len())]
            } else {
                let idx = rng.gen_range(0..keys.len().saturating_sub(1));
                (keys[idx] + keys[idx + 1]) / 2
            }
        })
        .collect();

    let mut query_idx = 0;
    bencher.bench_local(|| {
        let key = query_keys[query_idx % query_keys.len()];
        query_idx += 1;
        black_box(bloom.contains(black_box(key)))
    });
}

#[divan::bench(args = SIZES)]
fn bloom_range_query_small(bencher: Bencher, size: usize) {
    let keys = generate_smooth_u64(Some(size));
    let bloom = BloomFilter::new_with_keys(&keys, 1024, 0.01);
    let query_ranges = generate_query_ranges(&keys, 0.01, 1000);

    let mut query_idx = 0;
    bencher.bench_local(|| {
        let (start, end) = query_ranges[query_idx % query_ranges.len()];
        query_idx += 1;
        black_box(bloom.range_query(black_box(start), black_box(end)))
    });
}

#[divan::bench(args = SIZES)]
fn bloom_range_query_medium(bencher: Bencher, size: usize) {
    let keys = generate_smooth_u64(Some(size));
    let bloom = BloomFilter::new_with_keys(&keys, 1024, 0.01);
    let query_ranges = generate_query_ranges(&keys, 0.07, 1000);

    let mut query_idx = 0;
    bencher.bench_local(|| {
        let (start, end) = query_ranges[query_idx % query_ranges.len()];
        query_idx += 1;
        black_box(bloom.range_query(black_box(start), black_box(end)))
    });
}

#[divan::bench(args = SIZES)]
fn bloom_range_query_large(bencher: Bencher, size: usize) {
    let keys = generate_smooth_u64(Some(size));
    let bloom = BloomFilter::new_with_keys(&keys, 1024, 0.01);
    let query_ranges = generate_query_ranges(&keys, 0.4, 1000);

    let mut query_idx = 0;
    bencher.bench_local(|| {
        let (start, end) = query_ranges[query_idx % query_ranges.len()];
        query_idx += 1;
        black_box(bloom.range_query(black_box(start), black_box(end)))
    });
}