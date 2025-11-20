pub mod x_fast_trie;
pub mod binary_search_tree;
pub mod y_fast_trie;
pub mod utils;

pub use x_fast_trie::{XFastTrie, XFastLevel, XFastValue, RepNode};
pub use binary_search_tree::BinarySearchTreeGroup;
pub use y_fast_trie::YFastTrie;

pub type Key = u64;