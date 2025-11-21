use std::sync::{Arc, RwLock};
use crate::Key;
use crate::infix_store::InfixStore;

// #[derive(Debug, Default)]
// pub struct InfixStore;

#[derive(Debug, Default)]
pub struct BinarySearchTreeGroup {
    root: Option<Box<TreeNode>>,
}

#[derive(Clone, Debug)]
struct TreeNode {
    key: Key,
    left: Option<Box<TreeNode>>,
    right: Option<Box<TreeNode>>,
    infix_store: Option<Arc<RwLock<InfixStore>>>
}

impl BinarySearchTreeGroup {
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn new_with_keys(keys: &[Key]) -> Self {
        if keys.is_empty() {
            return Self { root: None };
        }

        let mut sorted_keys = keys.to_vec();
        sorted_keys.sort();

        let root = Self::top_down_bst_insertion(&sorted_keys, 0, sorted_keys.len() as isize - 1);
        Self { root }
    }

    fn top_down_bst_insertion(keys: &[Key], start: isize, end: isize) -> Option<Box<TreeNode>> {
        if start > end {
            return None;
        }

        let mid = ((start + end) / 2) as usize;
        let root = Box::new(TreeNode {
            key: keys[mid],
            left: Self::top_down_bst_insertion(keys, start, mid as isize - 1),
            right: Self::top_down_bst_insertion(keys, mid as isize + 1, end),
            infix_store: None,
        });
        Some(root)
    }

    pub fn insert(&mut self, key: Key) {
        Self::insert_recursive(&mut self.root, key);
    }

    fn insert_recursive(node: &mut Option<Box<TreeNode>>, key: Key) {

        match node {
            None => {
                *node = Some(Box::new(TreeNode {
                    key,
                    left: None,
                    right: None,
                    infix_store: None,
                }));
            }
            Some(n) => {
                if key < n.key {
                    Self::insert_recursive(&mut n.left, key);
                } else {
                    Self::insert_recursive(&mut n.right, key);
                }
            }
        }
    }

    pub fn contains(&self, key: Key) -> bool {
        Self::contains_recursive(&self.root, key)
    }

    fn contains_recursive(node: &Option<Box<TreeNode>>, key: Key) -> bool {
        match node {
            None => false,
            Some(n) => {
                if key == n.key {
                    true
                } else if key < n.key {
                    Self::contains_recursive(&n.left, key)
                } else {
                    Self::contains_recursive(&n.right, key)
                }
            }
        }
    }

    fn find_node_mu(node: &mut Option<Box<TreeNode>>, key: Key) -> Option<&mut TreeNode> {
        match node {
            None => None,
            Some(n) => {
                if key == n.key {
                    Some(n.as_mut())
                }
                else if key < n.key {
                    Self::find_node_mu(&mut n.left, key)
                } else {
                    Self::find_node_mu(&mut n.right, key)
                }
            }
        }
    }

    pub fn set_infix_store(&mut self, key: Key, infix_store: InfixStore) {
        if let Some(node) = Self::find_node_mu(&mut self.root, key) {
            node.infix_store = Some(Arc::new(RwLock::new(infix_store)));
        }
    }

    pub fn get_infix_store(&self, key: Key) -> Option<Arc<RwLock<InfixStore>>> {
        Self::get_infix_store_recursive(&self.root, key)
    }

    fn get_infix_store_recursive(node: &Option<Box<TreeNode>>, key: Key) -> Option<Arc<RwLock<InfixStore>>> {
        match node {
            None => None,
            Some(n) => {
                if key == n.key {
                    n.infix_store.clone()
                } else if key < n.key {
                    Self::get_infix_store_recursive(&n.left, key)
                } else {
                    Self::get_infix_store_recursive(&n.right, key)
                }
            }
        }
    }

    pub fn predecessor_infix_store(&self, key: Key) -> Option<Arc<RwLock<InfixStore>>> {
        Self::predecessor_store_recursive(&self.root, key, None)
    }

    pub fn predecessor(&self, key: Key) -> Option<Key> {
        Self::predecessor_recursive(&self.root, key, None)
    }

    fn predecessor_recursive(node: &Option<Box<TreeNode>>, key: Key, best: Option<Key>) -> Option<Key> {
        match node {
            None => best,
            Some(n) => {
                if n.key == key {
                    Some(n.key)
                } else if key < n.key {
                    Self::predecessor_recursive(&n.left, key, best)
                } else {
                    Self::predecessor_recursive(&n.right, key, Some(n.key))
                }
            }
        }
    }

    pub fn successor(&self, key: Key) -> Option<Key> {
        Self::successor_recursive(&self.root, key, None)
    }

    fn successor_recursive(node: &Option<Box<TreeNode>>, key: Key, best: Option<Key>) -> Option<Key> {
        match node {
            None => best,
            Some(n) => {
                if n.key == key {
                    Some(n.key)
                } else if key < n.key {
                    Self::successor_recursive(&n.left, key, Some(n.key))
                } else {
                    Self::successor_recursive(&n.right, key, best)
                }
            }
        }
    }

    fn predecessor_store_recursive(node: &Option<Box<TreeNode>>, key: Key, best: Option<Arc<RwLock<InfixStore>>>) -> Option<Arc<RwLock<InfixStore>>> {
        match node {
            None => best,
            Some(n) => {
                if n.key == key {
                    n.infix_store.clone().or(best)
                } else if key < n.key {
                    Self::predecessor_store_recursive(&n.left, key, best)
                } else {
                    Self::predecessor_store_recursive(&n.right, key, n.infix_store.clone())
                }
            }
        }
    }

    pub fn successor_infix_store(&self, key: Key) -> Option<Arc<RwLock<InfixStore>>> {
        Self::successor_store_recursive(&self.root, key, None)
    }

    fn successor_store_recursive(node: &Option<Box<TreeNode>>, key: Key, best: Option<Arc<RwLock<InfixStore>>>) -> Option<Arc<RwLock<InfixStore>>> {
        match node {
            None => best,
            Some(n) => {
                if n.key == key {
                    n.infix_store.clone().or(best)
                } else if key < n.key {
                    Self::successor_store_recursive(&n.left, key, n.infix_store.clone())
                } else {
                    Self::successor_store_recursive(&n.right, key, best)
                }
            }
        }
    }

    #[allow(dead_code)]
    fn min_key(node: &Option<Box<TreeNode>>) -> Option<Key> {
        match node {
            None => None,
            Some(n) => {
                if n.left.is_none() {
                    Some(n.key)
                } else {
                    Self::min_key(&n.left)
                }
            }
        }
    }

    #[allow(dead_code)]
    fn max_key(node: &Option<Box<TreeNode>>) -> Option<Key> {
        match node {
            None => None,
            Some(n) => {
                if n.right.is_none() {
                    Some(n.key)
                } else {
                    Self::max_key(&n.right)
                }
            }
        }
    }

    #[allow(dead_code)]
    fn min_node(node: &Option<Box<TreeNode>>) -> Option<&TreeNode> {
        match node {
            None => None,
            Some(n) => {
                if n.left.is_none() {
                    Some(n)
                } else {
                    Self::min_node(&n.left)
                }
            }
        }
    }

    #[allow(dead_code)]
    fn max_node(node: &Option<Box<TreeNode>>) -> Option<&TreeNode> {
        match node {
            None => None,
            Some(n) => {
                if n.right.is_none() {
                    Some(n)
                } else {
                    Self::max_node(&n.right)
                }
            }
        }
    }

    pub fn pretty_print(&self) {
        println!("\n=== Binary Search Tree ===");
        if self.root.is_none() {
            println!("  (empty tree)");
        } else {
            Self::print_tree(&self.root, "", true);
        }
        println!("=========================\n");
    }

    fn print_tree(node: &Option<Box<TreeNode>>, prefix: &str, is_tail: bool) {
        if let Some(n) = node {
            println!("{}{} {}", prefix, if is_tail { "└──" } else { "├──" }, n.key);

            let new_prefix = format!("{}{}", prefix, if is_tail { "    " } else { "│   " });

            if n.right.is_some() || n.left.is_some() {
                if n.right.is_some() {
                    Self::print_tree(&n.right, &new_prefix, false);
                }
                if n.left.is_some() {
                    Self::print_tree(&n.left, &new_prefix, true);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_construction() {
        let bst = BinarySearchTreeGroup::new_with_keys(&[1, 2, 3, 20, 30, 4, 5, 6, 7]);
        assert!(bst.contains(1));
        assert!(bst.contains(2));
        assert!(bst.contains(30));
        assert!(bst.contains(4));
        assert!(bst.contains(5));
        assert!(bst.contains(6));
        assert!(bst.contains(7));
        assert!(!bst.contains(8));
        assert!(!bst.contains(9));
        assert!(!bst.contains(10));
    }

    #[test]
    fn test_tree_insertion() {
        let mut bst = BinarySearchTreeGroup::new();
        bst.insert(1);
        bst.insert(2);
        bst.insert(3);
        bst.insert(20);
        bst.insert(30);
        bst.insert(4);
        bst.insert(5);
        bst.insert(6);
        bst.insert(7);
        assert!(bst.contains(1));
        assert!(bst.contains(2));
        assert!(bst.contains(3));
        assert!(bst.contains(20));
        assert!(bst.contains(30));
        assert!(bst.contains(4));
        assert!(bst.contains(5));
        assert!(bst.contains(6));
        assert!(bst.contains(7));
        assert!(!bst.contains(8));
        assert!(!bst.contains(9));
        assert!(!bst.contains(10));
    }

    #[test]
    fn test_predecessor_infix_store() {
        let mut bst = BinarySearchTreeGroup::new_with_keys(&[10, 20, 30, 40, 50]);

        bst.set_infix_store(10, InfixStore::default());
        bst.set_infix_store(20, InfixStore::default());
        bst.set_infix_store(30, InfixStore::default());
        bst.set_infix_store(40, InfixStore::default());
        bst.set_infix_store(50, InfixStore::default());

        // exact match returns own store
        let store_30 = bst.get_infix_store(30).unwrap();
        let pred_30 = bst.predecessor_infix_store(30).unwrap();
        assert!(Arc::ptr_eq(&store_30, &pred_30));

        // between keys returns predecessor's store
        let pred_35 = bst.predecessor_infix_store(35).unwrap();
        assert!(Arc::ptr_eq(&store_30, &pred_35));

        let store_20 = bst.get_infix_store(20).unwrap();
        let pred_25 = bst.predecessor_infix_store(25).unwrap();
        assert!(Arc::ptr_eq(&store_20, &pred_25));

        // before first key
        assert!(bst.predecessor_infix_store(5).is_none());

        // after last key
        let store_50 = bst.get_infix_store(50).unwrap();
        let pred_60 = bst.predecessor_infix_store(60).unwrap();
        assert!(Arc::ptr_eq(&store_50, &pred_60));
    }
}