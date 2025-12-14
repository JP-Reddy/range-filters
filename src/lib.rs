pub mod binary_search_tree;
pub mod bitmap;
pub mod bloom_filter;
pub mod data_gen;
pub mod diva;
pub mod grafite_filter;
pub mod infix_store;
pub mod utils;
pub mod x_fast_trie;
pub mod y_fast_trie;

pub use binary_search_tree::BinarySearchTreeGroup;
pub use bitmap::{get_bit, rank, select, set_bit, has_bits_in_range};
pub use bloom_filter::BloomFilter;
pub use diva::Diva;
pub use grafite_filter::GrafiteFilter;
pub use infix_store::InfixStore;
pub use x_fast_trie::{RepNode, XFastLevel, XFastTrie, XFastValue};
pub use y_fast_trie::YFastTrie;

pub type Key = u64;
pub const U64_BITS: usize = 64;
