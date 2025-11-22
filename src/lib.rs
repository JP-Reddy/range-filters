pub mod x_fast_trie;
pub mod binary_search_tree;
pub mod y_fast_trie;
pub mod utils;
pub mod infix_store;
pub mod bitmap;
pub mod diva;

pub use x_fast_trie::{XFastTrie, XFastLevel, XFastValue, RepNode};
pub use binary_search_tree::BinarySearchTreeGroup;
pub use y_fast_trie::YFastTrie;
pub use infix_store::InfixStore;
pub use bitmap::{set_bit, get_bit, rank, select};
pub use diva::Diva;

pub type Key = u64;