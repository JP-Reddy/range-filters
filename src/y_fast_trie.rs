use crate::x_fast_trie::XFastTrie;
use crate::binary_search_tree::BinarySearchTreeGroup;
use crate::binary_search_tree::InfixStore;
use crate::Key;
use std::sync::{Arc, RwLock};

pub struct YFastTrie {
    pub x_fast_trie: XFastTrie,
}

impl YFastTrie {
    pub fn new(no_levels: usize) -> Self {
        Self {
            x_fast_trie: XFastTrie::new(no_levels),
        }
    }

    pub fn new_with_keys(keys: &[Key], no_levels: usize) -> Self {
        if keys.is_empty() {
            return Self::new(no_levels);
        }

        // step 1: sort and dedup keys
        let mut sorted_keys = keys.to_vec();
        sorted_keys.sort();
        sorted_keys.dedup();

        
        let bst_group_size = no_levels.max(8);

        let mut x_fast_trie = XFastTrie::new(no_levels);

        // step 2: partition all keys into BST group chunks of size ~log U (e.g. 64 keys per group for 64 bit keys)
        for chunk_start in (0..sorted_keys.len()).step_by(bst_group_size) {
            let chunk_end = (chunk_start + bst_group_size).min(sorted_keys.len());
            let chunk = &sorted_keys[chunk_start..chunk_end];

            // boundary key is the first key of this chunk
            let boundary_key = chunk[0];

            // step 3: insert boundary key into x-fast trie
            x_fast_trie.insert(boundary_key);

            // step 4: create a balanced BST group with all keys in this chunk
            let bst_group = BinarySearchTreeGroup::new_with_keys(chunk);
            let bst_group_arc = Arc::new(RwLock::new(bst_group));

            // step 5: attach the BST group to the boundary representative
            if let Some(rep_node) = x_fast_trie.lookup(boundary_key) {
                if let Ok(mut rep) = rep_node.write() {
                    rep.bst_group = Some(bst_group_arc);
                }
            }
        }

        Self { x_fast_trie }
    }

    pub fn predecessor(&self, key: Key) -> Option<Key> {
        // find the boundary representative
        let rep_node = self.x_fast_trie.predecessor(key)?;
        let rep = rep_node.read().ok()?;

        // search within the BST group
        if let Some(bst_group) = &rep.bst_group {
            if let Ok(bst) = bst_group.read() {
                return bst.predecessor(key);
            }
        }

        Some(rep.key)
    }

    pub fn predecessor_infix_store(&self, key: Key) -> Option<Arc<RwLock<InfixStore>>> {
        // find boundary via x-fast trie
        let rep_node = self.x_fast_trie.predecessor(key)?;
        let rep = rep_node.read().ok()?;
  
        // get the BST group and call its predecessor_infix_store
        if let Some(bst_group) = &rep.bst_group {
            if let Ok(bst) = bst_group.read() {
                return bst.predecessor_infix_store(key);
            }
        }
        None
    }

    pub fn successor_infix_store(&self, key: Key) -> Option<Arc<RwLock<InfixStore>>> {
        // find boundary via x-fast trie
        let rep_node = self.x_fast_trie.successor(key)?;
        let rep = rep_node.read().ok()?;
  
        // get the BST group and call its successor_infix_store
        if let Some(bst_group) = &rep.bst_group {
            if let Ok(bst) = bst_group.read() {
                return bst.successor_infix_store(key);
            }
        }
        None
    }
    pub fn successor(&self, key: Key) -> Option<Key> {
        // find the boundary representative
        let rep_node = self.x_fast_trie.successor(key)?;
        let rep = rep_node.read().ok()?;

        // search within the BST group
        if let Some(bst_group) = &rep.bst_group {
            if let Ok(bst) = bst_group.read() {
                return bst.successor(key);
            }
        }

        Some(rep.key)
    }

    pub fn contains(&self, key: Key) -> bool {
        // first check x-fast trie for direct hit
        if self.x_fast_trie.lookup(key).is_some() {
            return true;
        }

        // find the predecessor boundary representative
        if let Some(rep_node) = self.x_fast_trie.predecessor(key) {
            if let Ok(rep) = rep_node.read() {
                // then check if key is in the BST group
                if let Some(bst_group) = &rep.bst_group {
                    if let Ok(bst) = bst_group.read() {
                        return bst.contains(key);
                    }
                }
            }
        }
        false
    }
}

