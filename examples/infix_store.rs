use range_filters::infix_store::InfixStore;

fn main() {
    let mut keys = vec![
        127u64 << 8 | 42,
        127u64 << 8 | 43,
        130u64 << 8 | 6,
        130u64 << 8 | 7,
        140u64 << 8 | 2,
        150u64 << 8 | 2,
        160u64 << 8 | 10,
    ];

    keys.sort();
    println!("keys: {:?}", keys);

    let infix_store = InfixStore::new_with_infixes(&keys, 8);
    println!("{}", infix_store);
    // println!("infix store: {:?}", infix_store);
}
