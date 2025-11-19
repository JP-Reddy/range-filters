pub mod x_fast_trie;
pub mod binary_search_tree;

pub use x_fast_trie::{XFastTrie, XFastLevel, XFastValue, RepNode};
pub use binary_search_tree::BinarySearchTree;

pub type Key = u64;