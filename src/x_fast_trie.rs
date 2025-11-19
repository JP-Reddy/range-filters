use std::sync::{Arc, RwLock, Weak};
use dashmap::DashMap;
use crate::Key;

pub const ROOT_KEY: Key = 67;

// placeholder
#[derive(Debug, Default)]
pub struct BSTGroup;

#[derive(Debug)]
pub struct XFastTrie {
    pub levels: Vec<XFastLevel>,
    // representatives
    // pub reps: HashMap<Key, Arc<RwLock<RepNode>>>,
    pub head_rep: Option<Arc<RwLock<RepNode>>>,
    pub tail_rep: Option<Arc<RwLock<RepNode>>>,
    
    // no. of levels = no. of bits in the keys
    pub no_levels: usize,
}

#[derive(Debug, Default, Clone)]
pub struct XFastLevel {
    pub table: DashMap<Key, XFastValue>
}

#[derive(Debug, Default, Clone)]
pub struct XFastValue {

    pub left_child: Option<Arc<RwLock<XFastValue>>>,
    pub right_child: Option<Arc<RwLock<XFastValue>>>,

    // pub representative: Option<Arc<RwLock<RepNode>>>
    pub min_rep: Option<Arc<RwLock<RepNode>>>,
    pub max_rep: Option<Arc<RwLock<RepNode>>>
}

#[derive(Debug, Default, Clone)]
pub struct RepNode {
    pub key: Key,
    pub left: Option<Weak<RwLock<RepNode>>>,
    pub right: Option<Weak<RwLock<RepNode>>>,
    pub bucket: Option<Arc<RwLock<BSTGroup>>>,
}

impl XFastTrie {
    pub fn new(no_levels: usize) -> Self {
        let mut levels = Vec::with_capacity(no_levels + 1);
        let root = XFastLevel::default();
        
        // insert the root level
        // use a random key for the root level
        root.table.insert(ROOT_KEY, XFastValue::default()); 
        levels.push(root);
        for _ in 1..=no_levels {
            let new_level = XFastLevel::default();
            levels.push(new_level);
        }
        Self {
            levels,
            head_rep: None,
            tail_rep: None,
            no_levels: no_levels,
        }
    }

    // fn contains(&self, key: Key) -> bool {
    //     self.reps.contains_key(&key)
    // }
    // find length of longest prefix of key
    fn find_longest_prefix_length(&self, key: Key) -> usize {

        // check if tree is empty
        if self.levels[1].table.is_empty() {
            return 0;
        }

        let mut low = 0;
        let mut high = self.no_levels;

        while low < high {
            let mid = (low + high + 1) / 2;
            let prefix = key >> (self.no_levels - mid);
            if self.levels[mid as usize].table.contains_key(&prefix) {
                // println!("prefix: {} found at level {}", prefix, mid);
                low = mid;
            } else {
                high = mid - 1;
                // println!("prefix: {} not found at level {}, searching in level {}", prefix, mid, high);
            }
        }

        low as usize
    }

    pub fn predecessor(&self, key: Key) -> Option<Arc<RwLock<RepNode>>> {

        // empty trie
        if self.levels[1].table.is_empty() {
            return None;
        }

        let longest_prefix_length = self.find_longest_prefix_length(key);

        // println!("longest_prefix_length: {}", longest_prefix_length);
        let prefix = key >> (self.no_levels - longest_prefix_length);

        let x_fast_value = self.levels[longest_prefix_length as usize].table.get(&prefix)?;

        if let Some(representative) = &x_fast_value.max_rep {
            if let Ok(rep) = representative.read() {
                if rep.key <= key {
                    return Some(representative.clone());
                }
            }
        }
        if let Some(representative) = &x_fast_value.min_rep {
            if let Ok(rep) = representative.read() {
                if rep.key <= key {
                    return Some(representative.clone());
                } else {
                    // need to find predecessor by traversing left
                    if let Some(left_weak) = &rep.left {
                        return left_weak.upgrade();
                    }
                    else {
                        return None;
                    }
                }
            }
        }

        None
    }

    pub fn successor(&self, key: Key) -> Option<Arc<RwLock<RepNode>>> {
        // empty trie
        if self.levels[1].table.is_empty() {
            return None;
        }

        let longest_prefix_length = self.find_longest_prefix_length(key);

        let prefix = key >> (self.no_levels - longest_prefix_length);

        let x_fast_value = self.levels[longest_prefix_length as usize].table.get(&prefix)?;

        if let Some(representative) = &x_fast_value.min_rep {
            if let Ok(rep) = representative.read() {
                if rep.key >= key {
                    return Some(representative.clone());
                }
            }
        }

        if let Some(representative) = &x_fast_value.max_rep {
            if let Ok(rep) = representative.read() {
                if rep.key >= key {
                    return Some(representative.clone());
                } else {
                    // need to find successor by traversing right
                    if let Some(right_weak) = &rep.right {
                        return right_weak.upgrade();
                    }
                    else {
                        return None;
                    }
                }
            }
        }

        None
    }

    // insert a key into the x-fast trie
    pub fn insert(&mut self, key: Key) {

        // step 1: find the longest prefix length 
        let longest_prefix_length = self.find_longest_prefix_length(key);

        println!("longest_prefix_length: {}", longest_prefix_length);
        
        let predecessor = self.predecessor(key);
        let successor = self.successor(key);

        // step 2: create representative
        let representative = Arc::new(RwLock::new(RepNode {
            key,
            left: None,
            right: None,
            bucket: None,
        }));

        // step 3: create child prefixes from longest_prefix_length+1 to no_levels
        for prefix_length in (longest_prefix_length+1)..=self.no_levels {
            let prefix = key >> (self.no_levels - prefix_length);
            let new_x_fast_value = XFastValue {
                left_child: None,
                right_child: None,
                min_rep: Some(representative.clone()),
                max_rep: Some(representative.clone()),
            };
            self.levels[prefix_length as usize].table.insert(prefix, new_x_fast_value.clone());

            // update parent's child pointers
            if prefix_length > 1 {
                let parent_prefix = key >> (self.no_levels - (prefix_length - 1));
                if let Some(mut parent_value) = self.levels[(prefix_length - 1) as usize].table.get_mut(&parent_prefix) {
                    let bit = (key >> (self.no_levels - prefix_length)) & 1;
                    if bit == 0 {
                        parent_value.left_child = Some(Arc::new(RwLock::new(new_x_fast_value.clone())));
                    } else {
                        parent_value.right_child = Some(Arc::new(RwLock::new(new_x_fast_value.clone())));
                    }
                }
            }
            else {
                // update root level child pointers
                if let Some(mut root_value) = self.levels[0].table.get_mut(&ROOT_KEY) {
                    let bit = key >> (self.no_levels - prefix_length);
                    if bit == 0 {
                        root_value.left_child = Some(Arc::new(RwLock::new(new_x_fast_value.clone())));
                    } else {
                        root_value.right_child = Some(Arc::new(RwLock::new(new_x_fast_value.clone())));
                    }
                }
            }
        }

        // step 4: update all prefixes' parents' min and max representatives
        if longest_prefix_length > 0 {
            for prefix_length in (1..=self.no_levels - 1).rev() {
                let prefix = key >> (self.no_levels - prefix_length);
                let mut x_fast_value = self.levels[prefix_length as usize].table.get_mut(&prefix).unwrap();

                let rep_key = representative.read().unwrap().key;

                let should_update_min = x_fast_value.min_rep.as_ref()
                    .and_then(|m| m.read().ok())
                    .map(|m| rep_key < m.key)
                    .unwrap_or(false);
                    
                let should_update_max = x_fast_value.max_rep.as_ref()
                    .and_then(|m| m.read().ok())
                    .map(|m| rep_key > m.key)
                    .unwrap_or(false);
                
                if should_update_min {
                    x_fast_value.min_rep = Some(representative.clone());
                }
                if should_update_max {
                    x_fast_value.max_rep = Some(representative.clone());
                }
            }
        }

        // step 5: update linked list pointers
        // update predecessor's right pointer
        if let Some(pred) = &predecessor {
            if let Ok(mut pred_guard) = pred.write() {
                pred_guard.right = Some(Arc::downgrade(&representative));
            }
        }
        
        // update successor's left pointer  
        if let Some(succ) = &successor {
            if let Ok(mut succ_guard) = succ.write() {
                succ_guard.left = Some(Arc::downgrade(&representative));
            }
        }
        
        // set representative's pointers
        if let Ok(mut rep_guard) = representative.write() {
            rep_guard.left = predecessor.as_ref().map(|p| Arc::downgrade(p));
            rep_guard.right = successor.map(|s| Arc::downgrade(&s));
            rep_guard.bucket = Some(Arc::new(RwLock::new(BSTGroup::default())));
        }

        // step 6: update head and tail representatives
        let should_update_head = if let Some(head_rep) = &self.head_rep {
            if let Ok(head) = head_rep.read() {
                head.key > key
            } else {
                false
            }
        } else {
            // first key being inserted
            true 
        };
        
        if should_update_head {
            self.head_rep = Some(representative.clone());
        }
        
        let should_update_tail = if let Some(tail_rep) = &self.tail_rep {
            if let Ok(tail) = tail_rep.read() {
                tail.key < key
            } else {
                false
            }
        } else {
            // first key being inserted
            true 
        };
        
        if should_update_tail {
            self.tail_rep = Some(representative.clone());
        }
    }

    pub fn pretty_print(&self) {
        println!("\n=== X-Fast Trie Structure ===");
        
        println!("\nRepresentatives (Linked List):");
        if let Some(head) = &self.head_rep {
            self.print_linked_list(head.clone());
        } else {
            println!("  Empty");
        }
        
        println!("\nTrie Levels:");
        for (level, x_fast_level) in self.levels.iter().enumerate() {
            if !x_fast_level.table.is_empty() {
                println!("  Level {} (prefix length {}):", level, level);
                let mut entries: Vec<_> = x_fast_level.table.iter().collect();
                entries.sort_by_key(|entry| *entry.key());
                
                for entry in entries {
                    let prefix = entry.key();
                    let value = entry.value();
                    let prefix_str = if level == 0 {
                        "ε".to_string()
                    } else {
                        format!("{:0width$b}", prefix, width = level)
                    };
                    
                    print!("    {}: ", prefix_str);
                    
                    if let Some(min_rep) = &value.min_rep {
                        if let Ok(rep_guard) = min_rep.read() {
                            print!("min_rep→{} ", rep_guard.key);
                        }
                    }
                    if let Some(max_rep) = &value.max_rep {
                        if let Ok(rep_guard) = max_rep.read() {
                            print!("max_rep→{} ", rep_guard.key);
                        }
                    }
                    if value.left_child.is_some() {
                        print!("L ");
                    }
                    if value.right_child.is_some() {
                        print!("R ");
                    }
                    
                    println!();
                }
            }
        }
        
        println!("\n=== End Structure ===\n");
    }
    
    fn print_linked_list(&self, start: Arc<RwLock<RepNode>>) {
        if let Ok(node) = start.read() {
            print!("  {} ", node.key);
            
            if let Some(right_weak) = &node.right {
                if let Some(right_arc) = right_weak.upgrade() {
                    print!("→ ");
                    self.print_linked_list_helper(right_arc);
                }
            }
            println!();
        }
    }
    
    fn print_linked_list_helper(&self, node: Arc<RwLock<RepNode>>) {
        if let Ok(node_guard) = node.read() {
            print!("{} ", node_guard.key);
            
            if let Some(right_weak) = &node_guard.right {
                if let Some(right_arc) = right_weak.upgrade() {
                    print!("→ ");
                    self.print_linked_list_helper(right_arc);
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_insert() {
        let mut trie = XFastTrie::new(8);
        trie.insert(42);
        
        // verify head and tail are set
        assert!(trie.head_rep.is_some());
        assert!(trie.tail_rep.is_some());
        
        if let Some(head) = &trie.head_rep {
            if let Ok(head_guard) = head.read() {
                assert_eq!(head_guard.key, 42);
            }
        }
    }

    #[test]
    fn test_multiple_inserts() {
        let mut trie = XFastTrie::new(8);
        let keys = vec![10, 5, 15, 3, 12];
        
        for key in &keys {
            trie.insert(*key);
        }
        
        // verify head is smallest, tail is largest
        if let Some(head) = &trie.head_rep {
            if let Ok(head_guard) = head.read() {
                assert_eq!(head_guard.key, 3);
            }
        }
        
        if let Some(tail) = &trie.tail_rep {
            if let Ok(tail_guard) = tail.read() {
                assert_eq!(tail_guard.key, 15);
            }
        }
    }

    #[test] 
    fn test_predecessor() {
        let mut trie = XFastTrie::new(8);
        let keys = vec![10, 20, 30, 40];
        
        for key in &keys {
            trie.insert(*key);
        }
        
        // test predecessor queries
        if let Some(pred) = trie.predecessor(25) {
            if let Ok(pred_guard) = pred.read() {
                assert_eq!(pred_guard.key, 20);
            }
        }
        
        if let Some(pred) = trie.predecessor(35) {
            if let Ok(pred_guard) = pred.read() {
                assert_eq!(pred_guard.key, 30);
            }
        }
        
        // test exact match
        if let Some(pred) = trie.predecessor(30) {
            if let Ok(pred_guard) = pred.read() {
                assert_eq!(pred_guard.key, 30);
            }
        }
    }

    #[test]
    fn test_successor() {
        let mut trie = XFastTrie::new(8);
        let keys = vec![10, 20, 30, 40];
        
        for key in &keys {
            trie.insert(*key);
        }
        
        // test successor queries
        if let Some(succ) = trie.successor(25) {
            if let Ok(succ_guard) = succ.read() {
                assert_eq!(succ_guard.key, 30);
            }
        }
        
        if let Some(succ) = trie.successor(15) {
            if let Ok(succ_guard) = succ.read() {
                assert_eq!(succ_guard.key, 20);
            }
        }
    }

    #[test]
    fn test_edge_cases() {
        let mut trie = XFastTrie::new(8);

        // predecessor of empty trie
        assert!(trie.predecessor(10).is_none());

        // insert single key
        trie.insert(50);

        // predecessor of smaller value
        assert!(trie.predecessor(10).is_none());

        // successor of larger value
        assert!(trie.successor(100).is_none());
    }

    // helper function to verify min/max representatives at a given level and prefix
    fn verify_min_max(trie: &XFastTrie, level: usize, prefix: Key, expected_min: Key, expected_max: Key) {
        let value = trie.levels[level].table.get(&prefix)
            .expect(&format!("prefix {} not found at level {}", prefix, level));

        if let Some(min_rep) = &value.min_rep {
            if let Ok(rep_guard) = min_rep.read() {
                assert_eq!(rep_guard.key, expected_min,
                    "Level {}, prefix {}: expected min_rep={}, got {}",
                    level, prefix, expected_min, rep_guard.key);
            }
        } else {
            panic!("Level {}, prefix {}: min_rep is None", level, prefix);
        }

        if let Some(max_rep) = &value.max_rep {
            if let Ok(rep_guard) = max_rep.read() {
                assert_eq!(rep_guard.key, expected_max,
                    "Level {}, prefix {}: expected max_rep={}, got {}",
                    level, prefix, expected_max, rep_guard.key);
            }
        } else {
            panic!("Level {}, prefix {}: max_rep is None", level, prefix);
        }
    }

    #[test]
    fn test_min_max_values_comprehensive() {
        let mut trie = XFastTrie::new(8);
        let keys = vec![10, 5, 15, 3, 12];

        for key in &keys {
            trie.insert(*key);
        }

        // Level 1
        verify_min_max(&trie, 1, 0b0, 3, 15);

        // Level 2
        verify_min_max(&trie, 2, 0b00, 3, 15);

        // Level 3
        verify_min_max(&trie, 3, 0b000, 3, 15);

        // Level 4
        verify_min_max(&trie, 4, 0b0000, 3, 15);

        // Level 5
        verify_min_max(&trie, 5, 0b00000, 3, 5);
        verify_min_max(&trie, 5, 0b00001, 10, 15);

        // Level 6
        verify_min_max(&trie, 6, 0b000000, 3, 3);
        verify_min_max(&trie, 6, 0b000001, 5, 5);
        verify_min_max(&trie, 6, 0b000010, 10, 10);
        verify_min_max(&trie, 6, 0b000011, 12, 15);

        // Level 7
        verify_min_max(&trie, 7, 0b0000001, 3, 3);
        verify_min_max(&trie, 7, 0b0000010, 5, 5);
        verify_min_max(&trie, 7, 0b0000101, 10, 10);
        verify_min_max(&trie, 7, 0b0000110, 12, 12);
        verify_min_max(&trie, 7, 0b0000111, 15, 15);

        // Level 8 (leaf level)
        verify_min_max(&trie, 8, 0b00000011, 3, 3);
        verify_min_max(&trie, 8, 0b00000101, 5, 5);
        verify_min_max(&trie, 8, 0b00001010, 10, 10);
        verify_min_max(&trie, 8, 0b00001100, 12, 12);
        verify_min_max(&trie, 8, 0b00001111, 15, 15);
    }

    #[test]
    fn test_min_max_single_key() {
        let mut trie = XFastTrie::new(8);
        trie.insert(42); // 42 = 0b00101010

        // all nodes should have min_rep=42 and max_rep=42
        // Level 1: prefix 0
        verify_min_max(&trie, 1, 0b0, 42, 42);

        // Level 2: prefix 00
        verify_min_max(&trie, 2, 0b00, 42, 42);

        // Level 3: prefix 001
        verify_min_max(&trie, 3, 0b001, 42, 42);

        // Level 4: prefix 0010
        verify_min_max(&trie, 4, 0b0010, 42, 42);

        // Level 5: prefix 00101
        verify_min_max(&trie, 5, 0b00101, 42, 42);

        // Level 6: prefix 001010
        verify_min_max(&trie, 6, 0b001010, 42, 42);

        // Level 7: prefix 0010101
        verify_min_max(&trie, 7, 0b0010101, 42, 42);

        // Level 8: prefix 00101010
        verify_min_max(&trie, 8, 0b00101010, 42, 42);
    }
    
    #[test]
    fn test_min_max_adjacent_keys() {
        let mut trie = XFastTrie::new(8);
        trie.insert(8);  // 0b00001000
        trie.insert(9);  // 0b00001001

        // these keys differ only in the last bit, so they share prefix up to level 7
        verify_min_max(&trie, 1, 0b0, 8, 9);
        verify_min_max(&trie, 2, 0b00, 8, 9);
        verify_min_max(&trie, 3, 0b000, 8, 9);
        verify_min_max(&trie, 4, 0b0000, 8, 9);
        verify_min_max(&trie, 5, 0b00001, 8, 9);
        verify_min_max(&trie, 6, 0b000010, 8, 9);
        verify_min_max(&trie, 7, 0b0000100, 8, 9);

        // leaf level - each key has its own entry
        verify_min_max(&trie, 8, 0b00001000, 8, 8);
        verify_min_max(&trie, 8, 0b00001001, 9, 9);
    }

    #[test]
    fn test_min_max_sequential_insertion() {
        let mut trie = XFastTrie::new(8);

        // insert in increasing order
        for key in [1, 2, 3, 4, 5] {
            trie.insert(key);
        }

        // verify that min is always 1 and max is always 5 at top levels
        // Level 1: all keys share prefix 0
        verify_min_max(&trie, 1, 0b0, 1, 5);

        // Level 8: each leaf has equal min and max
        verify_min_max(&trie, 8, 0b00000001, 1, 1);
        verify_min_max(&trie, 8, 0b00000010, 2, 2);
        verify_min_max(&trie, 8, 0b00000011, 3, 3);
        verify_min_max(&trie, 8, 0b00000100, 4, 4);
        verify_min_max(&trie, 8, 0b00000101, 5, 5);
    }

    #[test]
    fn test_min_max_reverse_insertion() {
        let mut trie = XFastTrie::new(8);

        // insert in decreasing order
        for key in [5, 4, 3, 2, 1] {
            trie.insert(key);
        }

        // min/max should be the same regardless of insertion order
        verify_min_max(&trie, 1, 0b0, 1, 5);

        // leaf level
        verify_min_max(&trie, 8, 0b00000001, 1, 1);
        verify_min_max(&trie, 8, 0b00000101, 5, 5);
    }

    #[test]
    fn test_min_max_sparse_keys() {
        let mut trie = XFastTrie::new(16);

        // insert sparse keys with large gaps
        trie.insert(1);   // 0b0000000000000001
        trie.insert(128); // 0b0000000010000000
        trie.insert(255); // 0b0000000011111111
        trie.insert(64);  // 0b0000000001000000

        // at level 1, keys all share prefix 0
        verify_min_max(&trie, 1, 0b0, 1, 255);

        // at level 9, keys diverge
        verify_min_max(&trie, 9, 0b000000000, 1, 64);
        verify_min_max(&trie, 9, 0b000000001, 128, 255);

        // leaf level - each key has its own entry
        verify_min_max(&trie, 16, 0b0000000000000001, 1, 1);
        verify_min_max(&trie, 16, 0b0000000001000000, 64, 64);
        verify_min_max(&trie, 16, 0b0000000010000000, 128, 128);
        verify_min_max(&trie, 16, 0b0000000011111111, 255, 255);
    }
}