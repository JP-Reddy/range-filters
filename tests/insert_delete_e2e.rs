use range_filters::diva::Diva;
use std::panic;

#[test]
fn test_basic_roundtrip() {
    let mut diva = Diva::new_with_keys(&[0, 10000], 1024, 0.01);
    assert!(diva.insert(5000));
    assert!(diva.delete(5000));
    assert!(!diva.delete(5000));
}

#[test]
fn test_bulk_operations() {
    let mut diva = Diva::new_with_keys(&[0, 100000], 1024, 0.01);
    let mut inserted = Vec::new();

    for key in 10000..11000 {
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| unsafe {
            (&mut *(&mut diva as *mut Diva)).insert(key)
        }));
        if let Ok(true) = result {
            inserted.push(key);
        }
    }

    for &key in &inserted {
        assert!(diva.delete(key));
    }

    for &key in &inserted {
        assert!(!diva.delete(key));
    }
}

#[test]
fn test_dense_insertion() {
    let mut diva = Diva::new_with_keys(&[0, 10000], 1024, 0.01);
    let mut inserted = Vec::new();

    for key in 5000..5200 {
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| unsafe {
            (&mut *(&mut diva as *mut Diva)).insert(key)
        }));
        if let Ok(true) = result {
            inserted.push(key);
        }
    }

    for &key in &inserted {
        assert!(diva.delete(key));
    }
}

#[test]
fn test_sparse_insertion() {
    let mut diva = Diva::new_with_keys(&[0, 1000000], 1024, 0.01);
    let keys = vec![1000, 50000, 100000, 500000, 900000];

    for &key in &keys {
        assert!(diva.insert(key));
    }

    for &key in &keys {
        assert!(diva.delete(key));
    }
}

#[test]
fn test_interleaved_ops() {
    let mut diva = Diva::new_with_keys(&[0, 100000], 1024, 0.01);

    assert!(diva.insert(10000));
    assert!(diva.insert(20000));
    assert!(diva.delete(10000));
    assert!(diva.insert(30000));
    assert!(diva.delete(20000));
    assert!(diva.insert(40000));
    assert!(diva.delete(30000));
    assert!(diva.delete(40000));
}

#[test]
fn test_boundary_cases() {
    let mut diva = Diva::new_with_keys(&[1000, 5000, 10000], 1024, 0.01);

    assert!(diva.insert(1001));
    assert!(diva.insert(4999));
    assert!(diva.insert(5001));
    assert!(diva.insert(9999));

    assert!(diva.delete(1001));
    assert!(diva.delete(4999));
    assert!(diva.delete(5001));
    assert!(diva.delete(9999));
}

#[test]
fn test_duplicate_handling() {
    let mut diva = Diva::new_with_keys(&[0, 10000], 1024, 0.01);

    assert!(diva.insert(5000));
    assert!(diva.insert(5000));
    assert!(diva.insert(5000));

    assert!(diva.delete(5000));
    assert!(!diva.delete(5000));
}

#[test]
fn test_partial_deletion() {
    let mut diva = Diva::new_with_keys(&[0, 100000], 1024, 0.01);
    let mut inserted = Vec::new();

    for key in 10000..10100 {
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| unsafe {
            (&mut *(&mut diva as *mut Diva)).insert(key)
        }));
        if let Ok(true) = result {
            inserted.push(key);
        }
    }

    let half = inserted.len() / 2;
    for &key in &inserted[..half] {
        assert!(diva.delete(key));
    }

    for key in 10100..10150 {
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| unsafe {
            (&mut *(&mut diva as *mut Diva)).insert(key)
        }));
        if let Ok(true) = result {
            inserted.push(key);
        }
    }

    for &key in &inserted[half..] {
        assert!(diva.delete(key));
    }
}

#[test]
fn test_reverse_deletion() {
    let mut diva = Diva::new_with_keys(&[0, 100000], 1024, 0.01);
    let mut inserted = Vec::new();

    for key in 10000..10050 {
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| unsafe {
            (&mut *(&mut diva as *mut Diva)).insert(key)
        }));
        if let Ok(true) = result {
            inserted.push(key);
        }
    }

    for &key in inserted.iter().rev() {
        assert!(diva.delete(key));
    }
}

#[test]
fn test_clustered_data() {
    let mut diva = Diva::new_with_keys(&[0, 10000, 20000, 30000, 40000, 50000], 1024, 0.01);
    let mut inserted = Vec::new();

    let clusters = vec![
        1000..1050,
        11000..11050,
        21000..21050,
        31000..31050,
        41000..41050,
    ];

    for cluster in clusters {
        for key in cluster {
            let result = panic::catch_unwind(panic::AssertUnwindSafe(|| unsafe {
                (&mut *(&mut diva as *mut Diva)).insert(key)
            }));
            if let Ok(true) = result {
                inserted.push(key);
            }
        }
    }

    for &key in &inserted {
        assert!(diva.delete(key));
    }
}

#[test]
fn test_out_of_bounds_below() {
    let mut diva = Diva::new_with_keys(&[1000, 10000], 1024, 0.01);
    let insert_result = panic::catch_unwind(panic::AssertUnwindSafe(|| unsafe {
        (&mut *(&mut diva as *mut Diva)).insert(500)
    }));
    if let Ok(true) = insert_result {
        let delete_result = panic::catch_unwind(panic::AssertUnwindSafe(|| unsafe {
            (&mut *(&mut diva as *mut Diva)).delete(500)
        }));
        assert!(matches!(delete_result, Ok(true)));
    }
}

#[test]
fn test_out_of_bounds_above() {
    let mut diva = Diva::new_with_keys(&[1000, 10000], 1024, 0.01);
    let insert_result = panic::catch_unwind(panic::AssertUnwindSafe(|| unsafe {
        (&mut *(&mut diva as *mut Diva)).insert(20000)
    }));
    if let Ok(true) = insert_result {
        let delete_result = panic::catch_unwind(panic::AssertUnwindSafe(|| unsafe {
            (&mut *(&mut diva as *mut Diva)).delete(20000)
        }));
        assert!(matches!(delete_result, Ok(true)));
    }
}

#[test]
fn test_insert_sample_key() {
    let mut diva = Diva::new_with_keys(&[1000, 5000, 10000], 1024, 0.01);
    assert!(diva.insert(1000));
    assert!(diva.insert(5000));
    assert!(diva.insert(10000));
}

#[test]
fn test_reinsert_after_delete() {
    let mut diva = Diva::new_with_keys(&[0, 10000], 1024, 0.01);
    assert!(diva.insert(5000));
    assert!(diva.delete(5000));
    assert!(diva.insert(5000));
    assert!(diva.delete(5000));
    assert!(!diva.delete(5000));
}

#[test]
fn test_reinsert_multiple_keys() {
    let mut diva = Diva::new_with_keys(&[0, 100000], 1024, 0.01);
    let keys = vec![1000, 2000, 3000];

    for &key in &keys {
        assert!(diva.insert(key));
    }
    for &key in &keys {
        assert!(diva.delete(key));
    }
    for &key in &keys {
        assert!(diva.insert(key));
    }
    for &key in &keys {
        assert!(diva.delete(key));
    }
}
