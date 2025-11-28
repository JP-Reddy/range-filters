use crate::Key;
use crate::binary_search_tree::BinarySearchTreeGroup;
use crate::infix_store::InfixStore;
use crate::x_fast_trie::XFastTrie;
use std::fmt;
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

        let bst_group_size = no_levels;

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

    pub fn len(&self) -> usize {
        let mut total = 0;
        if let Some(head) = &self.x_fast_trie.head_rep {
            let mut current = Some(head.clone());
            while let Some(node) = current {
                if let Ok(n) = node.read() {
                    if let Some(bst_group) = &n.bst_group {
                        if let Ok(bst) = bst_group.read() {
                            total += bst.len();
                        }
                    }
                    current = n.right.as_ref().and_then(|w| w.upgrade());
                } else {
                    break;
                }
            }
        }
        total
    }

    pub fn sample_count(&self) -> usize {
        self.x_fast_trie.len()
    }

    pub fn get_infix_store(&self, key: Key) -> Option<Arc<RwLock<InfixStore>>> {
        // find the boundary representative
        let rep_node = self.x_fast_trie.lookup(key)?;
        let rep = rep_node.read().ok()?;

        // get the BST group and call its get_infix_store
        if let Some(bst_group) = &rep.bst_group {
            if let Ok(bst) = bst_group.read() {
                return bst.get_infix_store(key);
            }
        }

        None
    }

    pub fn set_infix_store(&mut self, key: Key, infix_store: InfixStore) {
        // find the boundary representative
        if let Some(rep_node) = self.x_fast_trie.predecessor(key) {
            if let Ok(rep) = rep_node.read() {
                if let Some(bst_group) = &rep.bst_group {
                    if let Ok(mut bst) = bst_group.write() {
                        bst.set_infix_store(key, infix_store);
                    }
                }
            }
        }
    }

    // TODO: add insert method
    // TODO: add next, previous methods
    // TODO: create an iterator for the trie

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
        // find the containing bucket via predecessor boundary
        if let Some(rep_node) = self.x_fast_trie.predecessor(key) {
            if let Ok(rep) = rep_node.read() {
                // search within the BST group
                if let Some(bst_group) = &rep.bst_group {
                    if let Ok(bst) = bst_group.read() {
                        if let Some(result) = bst.successor_infix_store(key) {
                            return Some(result);
                        }
                    }
                }

                // key is > all keys in this bucket, try next bucket
                if let Some(next_weak) = &rep.right {
                    if let Some(next_rep) = next_weak.upgrade() {
                        if let Ok(next) = next_rep.read() {
                            if let Some(bst_group) = &next.bst_group {
                                if let Ok(bst) = bst_group.read() {
                                    return bst.get_infix_store(next.key);
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    pub fn successor(&self, key: Key) -> Option<Key> {
        // find the containing bucket via predecessor boundary
        if let Some(rep_node) = self.x_fast_trie.predecessor(key) {
            if let Ok(rep) = rep_node.read() {
                // search within the BST group
                if let Some(bst_group) = &rep.bst_group {
                    if let Ok(bst) = bst_group.read() {
                        if let Some(result) = bst.successor(key) {
                            return Some(result);
                        }
                    }
                }

                // key is > all keys in this bucket, try next bucket
                if let Some(next_weak) = &rep.right {
                    if let Some(next_rep) = next_weak.upgrade() {
                        if let Ok(next) = next_rep.read() {
                            return Some(next.key);
                        }
                    }
                }
            }
        } else {
            // key < first boundary, return first key
            if let Some(head) = &self.x_fast_trie.head_rep {
                if let Ok(head_guard) = head.read() {
                    return Some(head_guard.key);
                }
            }
        }

        None
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

    pub fn pretty_print(&self) {
        print!("{}", self);
    }

    // helper to collect all keys from BST in sorted order
    fn collect_bst_keys(node: &Option<Box<crate::binary_search_tree::TreeNode>>) -> Vec<Key> {
        let mut keys = Vec::new();
        Self::collect_bst_keys_recursive(node, &mut keys);
        keys
    }

    fn collect_bst_keys_recursive(
        node: &Option<Box<crate::binary_search_tree::TreeNode>>,
        keys: &mut Vec<Key>,
    ) {
        if let Some(n) = node {
            Self::collect_bst_keys_recursive(&n.left, keys);
            keys.push(n.key);
            Self::collect_bst_keys_recursive(&n.right, keys);
        }
    }
}

impl fmt::Display for YFastTrie {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "\n╔════════════════════════════════════════════════════════╗"
        )?;
        writeln!(
            f,
            "║             Y-FAST TRIE STRUCTURE                      ║"
        )?;
        writeln!(
            f,
            "╚════════════════════════════════════════════════════════╝"
        )?;

        // print stats
        writeln!(f, "\nStats:")?;
        writeln!(f, "  Total keys:        {}", self.len())?;
        writeln!(f, "  Sample count:      {}", self.sample_count())?;
        writeln!(f, "  Levels:            {}", self.x_fast_trie.no_levels)?;

        // print x-fast trie structure
        write!(f, "{}", self.x_fast_trie)?;

        // print BST groups for each representative
        writeln!(
            f,
            "\n╔════════════════════════════════════════════════════════╗"
        )?;
        writeln!(
            f,
            "║             BST GROUPS (per representative)            ║"
        )?;
        writeln!(
            f,
            "╚════════════════════════════════════════════════════════╝\n"
        )?;

        if let Some(head) = &self.x_fast_trie.head_rep {
            let mut current = Some(head.clone());
            let mut bucket_index = 0;

            while let Some(node) = current {
                if let Ok(n) = node.read() {
                    writeln!(f, "Bucket {} (representative: {})", bucket_index, n.key)?;

                    if let Some(bst_group) = &n.bst_group {
                        if let Ok(bst) = bst_group.read() {
                            write!(f, "{}", bst)?;

                            // check for InfixStores attached to keys in this BST
                            let keys = Self::collect_bst_keys(&bst.root);
                            let mut infix_stats = Vec::new();

                            for &key in &keys {
                                if let Some(infix_store_arc) = bst.get_infix_store(key) {
                                    if let Ok(infix_store) = infix_store_arc.read() {
                                        infix_stats.push((
                                            key,
                                            infix_store.elem_count(),
                                            infix_store.remainder_size(),
                                            infix_store.num_slots(),
                                        ));
                                    }
                                }
                            }

                            if !infix_stats.is_empty() {
                                writeln!(f, "  InfixStores:")?;
                                for (key, elem_count, remainder_size, num_slots) in infix_stats {
                                    writeln!(
                                        f,
                                        "    Key {}: {} elements, {} bit remainder, {} slots",
                                        key, elem_count, remainder_size, num_slots
                                    )?;
                                }
                            }
                        }
                    } else {
                        writeln!(f, "  (no BST group attached)")?;
                    }

                    current = n.right.as_ref().and_then(|w| w.upgrade());
                    bucket_index += 1;
                } else {
                    break;
                }
            }
        } else {
            writeln!(f, "  (no buckets)")?;
        }

        writeln!(
            f,
            "\n╔════════════════════════════════════════════════════════╗"
        )?;
        writeln!(
            f,
            "║                    END Y-FAST TRIE                     ║"
        )?;
        writeln!(
            f,
            "╚════════════════════════════════════════════════════════╝\n"
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_key() {
        let trie = YFastTrie::new_with_keys(&[42], 8);
        assert!(trie.contains(42));
    }

    #[test]
    fn test_basic_contains() {
        let keys = vec![10, 20, 30, 40, 50, 60, 70, 80];
        let trie = YFastTrie::new_with_keys(&keys, 8);

        for &key in &keys {
            assert!(trie.contains(key), "key {} should be in trie", key);
        }

        assert!(!trie.contains(5));
        assert!(!trie.contains(15));
        assert!(!trie.contains(85));
    }

    #[test]
    fn test_large_set() {
        // create 100 keys: 0, 10, 20, ..., 990
        let keys: Vec<Key> = (0..100).map(|i| i * 10).collect();
        let trie = YFastTrie::new_with_keys(&keys, 16);

        // verify all keys exist
        for &key in &keys {
            assert!(trie.contains(key), "key {} should exist", key);
        }

        // verify non-existent keys
        assert!(!trie.contains(5));
        assert!(!trie.contains(15));
        assert!(!trie.contains(995));
    }

    #[test]
    fn test_boundary_keys() {
        // with bst_group_size=8, these keys create 5 groups with boundaries: 0, 8, 16, 24, 32
        let keys: Vec<Key> = (0..40).collect();
        let trie = YFastTrie::new_with_keys(&keys, 8);

        // verify boundary keys are in x-fast
        assert!(trie.x_fast_trie.lookup(0).is_some());
        assert!(trie.x_fast_trie.lookup(8).is_some());
        assert!(trie.x_fast_trie.lookup(16).is_some());
        assert!(trie.x_fast_trie.lookup(24).is_some());
        assert!(trie.x_fast_trie.lookup(32).is_some());

        // verify non-boundary keys are NOT in x-fast
        assert!(trie.x_fast_trie.lookup(1).is_none());
        assert!(trie.x_fast_trie.lookup(9).is_none());
        assert!(trie.x_fast_trie.lookup(17).is_none());

        // but all keys should be in the trie overall
        for key in 0..40 {
            assert!(trie.contains(key), "key {} should be in trie", key);
        }
    }

    #[test]
    fn test_predecessor() {
        let keys = vec![10, 20, 30, 40, 50];
        let trie = YFastTrie::new_with_keys(&keys, 8);

        // exact matches
        assert_eq!(trie.predecessor(10), Some(10));
        assert_eq!(trie.predecessor(30), Some(30));
        assert_eq!(trie.predecessor(50), Some(50));

        // between keys
        assert_eq!(trie.predecessor(15), Some(10));
        assert_eq!(trie.predecessor(25), Some(20));
        assert_eq!(trie.predecessor(35), Some(30));
        assert_eq!(trie.predecessor(45), Some(40));

        // before first key
        assert_eq!(trie.predecessor(5), None);

        // after last key
        assert_eq!(trie.predecessor(60), Some(50));
    }

    #[test]
    fn test_successor() {
        let keys = vec![10, 20, 30, 40, 50];
        let trie = YFastTrie::new_with_keys(&keys, 8);

        // exact matches
        assert_eq!(trie.successor(10), Some(10));
        assert_eq!(trie.successor(30), Some(30));
        assert_eq!(trie.successor(50), Some(50));

        // between keys
        assert_eq!(trie.successor(15), Some(20));
        assert_eq!(trie.successor(25), Some(30));
        assert_eq!(trie.successor(35), Some(40));
        assert_eq!(trie.successor(45), Some(50));

        // before first key
        assert_eq!(trie.successor(5), Some(10));

        // after last key
        assert_eq!(trie.successor(60), None);
    }

    #[test]
    fn test_predecessor_successor_across_boundaries() {
        // 40 keys create boundaries at: 0, 8, 16, 24, 32
        let keys: Vec<Key> = (0..40).collect();
        let trie = YFastTrie::new_with_keys(&keys, 8);

        // test across BST group boundaries
        assert_eq!(trie.predecessor(7), Some(7));
        assert_eq!(trie.predecessor(8), Some(8));
        assert_eq!(trie.predecessor(9), Some(9));

        assert_eq!(trie.successor(7), Some(7));
        assert_eq!(trie.successor(8), Some(8));
        assert_eq!(trie.successor(9), Some(9));

        // between groups
        assert_eq!(trie.predecessor(15), Some(15));
        assert_eq!(trie.successor(15), Some(15));

        assert_eq!(trie.predecessor(16), Some(16));
        assert_eq!(trie.successor(16), Some(16));

        assert_eq!(trie.predecessor(17), Some(17));
        assert_eq!(trie.successor(17), Some(17));
    }

    #[test]
    fn test_infix_stores() {
        use crate::infix_store::InfixStore;

        // 24 keys: boundaries at 0, 24, 48
        let keys: Vec<Key> = (0..24).map(|i| i * 3).collect();
        let trie = YFastTrie::new_with_keys(&keys, 8);

        // attach infix stores to some keys across buckets
        let store_6 = InfixStore::default();
        let store_12 = InfixStore::default();
        let store_30 = InfixStore::default();

        // manually set infix stores in BST groups
        if let Some(rep) = trie.x_fast_trie.lookup(0) {
            if let Ok(r) = rep.read() {
                if let Some(bst_group) = &r.bst_group {
                    if let Ok(mut bst) = bst_group.write() {
                        bst.set_infix_store(6, store_6);
                    }
                }
            }
        }

        if let Some(rep) = trie.x_fast_trie.lookup(0) {
            if let Ok(r) = rep.read() {
                if let Some(bst_group) = &r.bst_group {
                    if let Ok(mut bst) = bst_group.write() {
                        bst.set_infix_store(12, store_12);
                    }
                }
            }
        }

        if let Some(rep) = trie.x_fast_trie.lookup(24) {
            if let Ok(r) = rep.read() {
                if let Some(bst_group) = &r.bst_group {
                    if let Ok(mut bst) = bst_group.write() {
                        bst.set_infix_store(30, store_30);
                    }
                }
            }
        }

        // get reference stores for comparison
        let ref_store_6 = {
            let rep = trie.x_fast_trie.lookup(0).unwrap();
            let r = rep.read().unwrap();
            let bst_group = r.bst_group.as_ref().unwrap();
            let bst = bst_group.read().unwrap();
            bst.get_infix_store(6).unwrap()
        };

        let ref_store_12 = {
            let rep = trie.x_fast_trie.lookup(0).unwrap();
            let r = rep.read().unwrap();
            let bst_group = r.bst_group.as_ref().unwrap();
            let bst = bst_group.read().unwrap();
            bst.get_infix_store(12).unwrap()
        };

        let ref_store_30 = {
            let rep = trie.x_fast_trie.lookup(24).unwrap();
            let r = rep.read().unwrap();
            let bst_group = r.bst_group.as_ref().unwrap();
            let bst = bst_group.read().unwrap();
            println!("bst: {:?}", bst);
            bst.get_infix_store(30).unwrap()
        };

        assert!(Arc::ptr_eq(
            &trie.predecessor_infix_store(8).unwrap(),
            &ref_store_6
        ));
        assert!(Arc::ptr_eq(
            &trie.predecessor_infix_store(12).unwrap(),
            &ref_store_12
        ));
        assert!(Arc::ptr_eq(
            &trie.predecessor_infix_store(31).unwrap(),
            &ref_store_30
        ));
        // assert!(Arc::ptr_eq(&trie.predecessor_infix_store(100).unwrap(), &ref_store_30));

        assert!(Arc::ptr_eq(
            &trie.successor_infix_store(5).unwrap(),
            &ref_store_6
        ));
        assert!(Arc::ptr_eq(
            &trie.successor_infix_store(10).unwrap(),
            &ref_store_12
        ));
        assert!(Arc::ptr_eq(
            &trie.successor_infix_store(29).unwrap(),
            &ref_store_30
        ));

        // extreme ends
        assert!(trie.predecessor_infix_store(2).is_none());
        assert!(trie.successor_infix_store(1000).is_none());
    }
}
