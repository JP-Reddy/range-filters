use crate::Key;

pub struct BinarySearchTree {
    root: Option<Box<TreeNode>>,
}

#[derive(Clone)]
struct TreeNode {
    key: Key,
    left: Option<Box<TreeNode>>,
    right: Option<Box<TreeNode>>,
}

impl BinarySearchTree {
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
        let bst = BinarySearchTree::new_with_keys(&[1, 2, 3, 20, 30, 4, 5, 6, 7]);
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
        let mut bst = BinarySearchTree::new();
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
}