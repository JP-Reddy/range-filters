use std::sync::{Arc, RwLock, Weak};
use dashmap::DashMap;

pub type Key = u64;

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
    pub no_levels: u8,
}

#[derive(Debug, Default, Clone)]
pub struct XFastLevel {
    pub table: DashMap<u64, XFastValue>
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
    pub fn new(no_levels: u8) -> Self {
        Self {
            levels: vec![XFastLevel::default(); no_levels as usize + 1],
            head_rep: None,
            tail_rep: None,
            no_levels,
        }
    }

    // fn contains(&self, key: Key) -> bool {
    //     self.reps.contains_key(&key)
    // }

    fn predecessor(&self, key: Key) -> Option<Arc<RwLock<RepNode>>> {
        let mut low = 0;
        let mut high = self.no_levels;

        while low < high {
            let mid = (low + high + 1) / 2;
            let prefix = key >> (self.no_levels - mid);
            if self.levels[mid as usize].table.contains_key(&prefix) {
                low = mid;
            }
            else {
                high = mid - 1;
            }
        }

        let best_level = low;

        if best_level == 0 {
            return self.head_rep.clone();
        }

        let prefix = key >> (self.no_levels - best_level);


        let x_fast_value = self.levels[best_level as usize].table.get(&prefix)?;

        if let Some(representative) = &x_fast_value.representative {
            if let Ok(rep) = representative.read() {
                if rep.key <= key {
                    return Some(representative.clone());
                } else {
                    // need to find predecessor by traversing left
                    if let Some(left_weak) = &rep.left {
                        return left_weak.upgrade();
                    }
                }
            }
        }

        None
    }

    fn successor(&self, key: Key) -> Option<Arc<RwLock<RepNode>>> {
        let mut low = 0;
        let mut high = self.no_levels;

        while low < high {
            let mid = (low + high + 1) / 2;
            let prefix = key >> (self.no_levels - mid);
            if self.levels[mid as usize].table.contains_key(&prefix) {
                low = mid;
            }
            else {
                high = mid - 1;
            }
        }

        let best_level = low;

        if best_level == 0 {
            return self.tail_rep.clone();
        }

        let prefix = key >> (self.no_levels - best_level);


        let x_fast_value = self.levels[best_level as usize].table.get(&prefix)?;

        if let Some(representative) = &x_fast_value.representative {
            if let Ok(rep) = representative.read() {
                if rep.key >= key {
                    return Some(representative.clone());
                } else {
                    // need to find successor by traversing right
                    if let Some(right_weak) = &rep.right {
                        return right_weak.upgrade();
                    }
                }
            }
        }

        None
    }
}