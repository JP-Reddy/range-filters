use range_filters::U64_BITS;
use range_filters::data_gen::generate_smooth_u64;
use range_filters::y_fast_trie::YFastTrie;

fn main() {
    let mut keys = generate_smooth_u64(Some(1000));
    keys.sort();
    println!("keys: {:?}", keys);

    let y_fast_trie = YFastTrie::new_with_keys(&keys, U64_BITS);

    // y_fast_trie.pretty_print();
    println!("Keys 99, 100, 101: {:?}", &keys[99..102]);
    println!(
        "key {} contains: {}",
        keys[100],
        y_fast_trie.contains(keys[100])
    );
    println!(
        "key {} predecessor: {:?}",
        keys[100],
        y_fast_trie.predecessor(keys[100] - 1)
    );
    println!(
        "key {} successor: {:?}",
        keys[100],
        y_fast_trie.successor(keys[100] + 1)
    );
    // println!("y-fast trie: {:?}", y_fast_trie);

    let keys = (10..2000).into_iter().step_by(10).collect::<Vec<_>>();
    println!("keys: {:?}", keys);
    let trie = YFastTrie::new_with_keys(&keys, 16);

    println!("{}", trie);
}
