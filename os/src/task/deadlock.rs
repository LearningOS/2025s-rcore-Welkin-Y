use alloc::collections::BTreeMap;
use alloc::vec;
use core::fmt::{self, Debug, Formatter};

#[allow(dead_code)]
pub struct DeadlockManager {
    // {key: resource id, val: (avail, {key: tid, val: (alloc, need)})}
    avail: BTreeMap<usize, usize>,
    need: BTreeMap<usize, BTreeMap<usize, usize>>,
    alloc: BTreeMap<usize, BTreeMap<usize, usize>>,
}

#[allow(dead_code)]
impl DeadlockManager {
    pub fn new() -> Self {
        Self {
            avail: BTreeMap::new(),
            need: BTreeMap::new(),
            alloc: BTreeMap::new(),
        }
    }

    /// add a resource to the graph
    pub fn add_resource(&mut self, rid: usize, avail: usize) {
        self.avail.insert(rid, avail);
    }

    /// add need
    pub fn add_need(&mut self, rid: usize, tid: usize, need: usize) {
        self.need
            .entry(tid)
            .or_default()
            .entry(rid)
            .and_modify(|x| *x += need)
            .or_insert(need);
    }

    /// alocate a resource to a thread
    pub fn add_alloc(&mut self, rid: usize, tid: usize) {
        self.avail.entry(rid).and_modify(|x| *x -= 1);
        self.alloc
            .entry(tid)
            .or_default()
            .entry(rid)
            .and_modify(|x| *x += 1)
            .or_insert(1);
        self.need.entry(tid).and_modify(|x| {
            if let Some(v) = x.get_mut(&rid) {
                *v -= 1;
            } else {
                panic!("Alloc should not be called before need");
            }
        });
    }

    pub fn add_release(&mut self, rid: usize, tid: usize) {
        self.avail.entry(rid).and_modify(|x| *x += 1);
        self.alloc.entry(tid).and_modify(|x| {
            if let Some(v) = x.get_mut(&rid) {
                *v -= 1;
                if *v == 0 {
                    x.remove(&rid);
                }
            }
        });
    }

    pub fn is_safe(&self, num_task: usize) -> bool {
        let mut finished = vec![false; num_task];
        let mut work = self.avail.clone();
        let mut next_scan = true;
        let mut need = self.need.clone();
        let alloc = self.alloc.clone();
        // println!("work: {:?}", work);
        // println!("need: {:?}", need);
        // println!("alloc: {:?}", alloc);
        while next_scan {
            next_scan = false;
            for (i, finish) in finished.iter_mut().enumerate() {
                if !(*finish) {
                    let mut can_finish = true;
                    for rid in 0..self.avail.len() {
                        if need.entry(i).or_default().entry(rid).or_default()
                            > work.entry(rid).or_default()
                        {
                            can_finish = false;
                            break;
                        }
                    }

                    if can_finish {
                        *finish = true;
                        next_scan = true;
                        if let Some(alloc) = alloc.get(&i) {
                            for (rid, alloc) in alloc {
                                if let Some(work) = work.get_mut(rid) {
                                    *work += alloc;
                                }
                            }
                        }
                    }
                }
            }
        }

        return finished.iter().all(|x| *x);
    }
}

impl Default for DeadlockManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for DeadlockManager {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "avail {:?}\n alloc {:?}\n need {:?}",
            self.avail, self.alloc, self.need
        ))
    }
}
