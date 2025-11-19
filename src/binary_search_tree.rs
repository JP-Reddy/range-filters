
type Key = u64;

struct BinarySearchTree {
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
}