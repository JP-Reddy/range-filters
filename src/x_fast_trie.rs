use std::sync::{Arc, RwLock, Weak};
use dashmap::DashMap;

pub type Key = u64;

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

    pub representative: Option<Arc<RwLock<RepNode>>>
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

        if let Some(representative) = &x_fast_value.representative {
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

        if let Some(representative) = &x_fast_value.representative {
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
                representative: Some(representative.clone()),
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

        // step 4: update linked list pointers
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

        // step 5: update head and tail representatives
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
                    
                    if let Some(rep) = &value.representative {
                        if let Ok(rep_guard) = rep.read() {
                            print!("rep→{} ", rep_guard.key);
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
}