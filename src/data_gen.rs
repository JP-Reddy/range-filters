use rand::Rng;
use rand::thread_rng;
use rand_distr::{Distribution, Normal, Uniform};

// default = 64k keys
const DEFAULT_COUNT: usize = 1 << 16;

pub fn generate_normal_u64(count: usize, mean: f64, std_dev: f64) -> Vec<u64> {
    let normal = Normal::new(mean, std_dev).unwrap();
    let mut rng = thread_rng();

    (0..count)
        .map(|_| {
            let sample: f64 = normal.sample(&mut rng);
            sample.max(0.0).min(u64::MAX as f64) as u64
        })
        .collect()
}

pub fn generate_normal_u32(count: usize, mean: f64, std_dev: f64) -> Vec<u32> {
    let normal = Normal::new(mean, std_dev).unwrap();
    let mut rng = thread_rng();

    (0..count)
        .map(|_| {
            let sample: f64 = normal.sample(&mut rng);
            sample.max(0.0).min(u32::MAX as f64) as u32
        })
        .collect()
}

pub fn generate_normal_i32(count: usize, mean: f64, std_dev: f64) -> Vec<i32> {
    let normal = Normal::new(mean, std_dev).unwrap();
    let mut rng = thread_rng();

    (0..count)
        .map(|_| {
            let sample: f64 = normal.sample(&mut rng);
            sample.max(i32::MIN as f64).min(i32::MAX as f64) as i32
        })
        .collect()
}

pub fn generate_normal_u16(count: usize, mean: f64, std_dev: f64) -> Vec<u16> {
    let normal = Normal::new(mean, std_dev).unwrap();
    let mut rng = thread_rng();

    (0..count)
        .map(|_| {
            let sample: f64 = normal.sample(&mut rng);
            sample.max(0.0).min(u16::MAX as f64) as u16
        })
        .collect()
}

pub fn generate_normal_u8(count: usize, mean: f64, std_dev: f64) -> Vec<u8> {
    let normal = Normal::new(mean, std_dev).unwrap();
    let mut rng = thread_rng();

    (0..count)
        .map(|_| {
            let sample: f64 = normal.sample(&mut rng);
            sample.max(0.0).min(u8::MAX as f64) as u8
        })
        .collect()
}

pub fn generate_uniform_u64(count: usize, min: u64, max: u64) -> Vec<u64> {
    let uniform = Uniform::new_inclusive(min, max);
    let mut rng = thread_rng();

    (0..count).map(|_| uniform.sample(&mut rng)).collect()
}

pub fn generate_uniform_u32(count: usize, min: u32, max: u32) -> Vec<u32> {
    let uniform = Uniform::new_inclusive(min, max);
    let mut rng = thread_rng();

    (0..count).map(|_| uniform.sample(&mut rng)).collect()
}

pub fn generate_uniform_i32(count: usize, min: i32, max: i32) -> Vec<i32> {
    let uniform = Uniform::new_inclusive(min, max);
    let mut rng = thread_rng();

    (0..count).map(|_| uniform.sample(&mut rng)).collect()
}

pub fn generate_strings(count: usize, min_len: usize, max_len: usize) -> Vec<String> {
    let mut rng = thread_rng();
    let len_dist = Uniform::new_inclusive(min_len, max_len);

    (0..count)
        .map(|_| {
            let len = len_dist.sample(&mut rng);
            (0..len)
                .map(|_| rng.gen_range(b'a'..=b'z') as char)
                .collect()
        })
        .collect()
}

// generate smooth u64 keys with normal distribution
pub fn generate_smooth_u64(count: Option<usize>) -> Vec<u64> {
    let count = count.unwrap_or(DEFAULT_COUNT);
    let mean = (u64::MAX / 2) as f64;
    let std_dev = (u64::MAX / 6) as f64;
    generate_normal_u64(count, mean, std_dev)
}

pub fn generate_smooth_u32(count: Option<usize>) -> Vec<u32> {
    let count = count.unwrap_or(DEFAULT_COUNT);
    let mean = (u32::MAX / 2) as f64;
    let std_dev = (u32::MAX / 6) as f64;
    generate_normal_u32(count, mean, std_dev)
}

pub fn generate_smooth_i32(count: Option<usize>) -> Vec<i32> {
    let count = count.unwrap_or(DEFAULT_COUNT);
    let mean = 0.0;
    let std_dev = (i32::MAX / 3) as f64;
    generate_normal_i32(count, mean, std_dev)
}

pub fn generate_smooth_u16(count: Option<usize>) -> Vec<u16> {
    let count = count.unwrap_or(DEFAULT_COUNT);
    let mean = (u16::MAX / 2) as f64;
    let std_dev = (u16::MAX / 6) as f64;
    generate_normal_u16(count, mean, std_dev)
}

pub fn generate_smooth_u8(count: Option<usize>) -> Vec<u8> {
    let count = count.unwrap_or(DEFAULT_COUNT);
    let mean = (u8::MAX / 2) as f64;
    let std_dev = (u8::MAX / 6) as f64;
    generate_normal_u8(count, mean, std_dev)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smooth_u64_default() {
        let data = generate_smooth_u64(None);
        assert_eq!(data.len(), DEFAULT_COUNT);
    }

    #[test]
    fn test_smooth_u64_custom_count() {
        let data = generate_smooth_u64(Some(1000));
        assert_eq!(data.len(), 1000);
    }

    #[test]
    fn test_normal_u64_bounds() {
        let data = generate_normal_u64(1000, 100.0, 10.0);
        assert_eq!(data.len(), 1000);
        assert!(data.iter().all(|&x| x < u64::MAX));
    }

    #[test]
    fn test_uniform_u64() {
        let data = generate_uniform_u64(1000, 0, 1000);
        assert_eq!(data.len(), 1000);
        assert!(data.iter().all(|&x| x <= 1000));
    }

    #[test]
    fn test_strings() {
        let data = generate_strings(100, 5, 10);
        assert_eq!(data.len(), 100);
        assert!(data.iter().all(|s| s.len() >= 5 && s.len() <= 10));
        assert!(
            data.iter()
                .all(|s| s.chars().all(|c| c.is_ascii_lowercase()))
        );
    }
}
