use std::collections::{HashSet, VecDeque};
use std::sync::Mutex;

pub struct LoopDetector {
    max_correlation_length: usize,
    recent_paths: Mutex<VecDeque<Vec<String>>>,
    max_recent: usize,
    loop_count: Mutex<u64>,
}

impl Default for LoopDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl LoopDetector {
    pub fn new() -> Self {
        Self {
            max_correlation_length: 16,
            recent_paths: Mutex::new(VecDeque::with_capacity(128)),
            max_recent: 128,
            loop_count: Mutex::new(0),
        }
    }

    pub fn with_max_correlation(mut self, max_corr: usize) -> Self {
        self.max_correlation_length = max_corr;
        self
    }

    pub fn record_hop(&self, path: &[String]) -> Option<Vec<String>> {
        if path.len() > self.max_correlation_length {
            let mut c = self.loop_count.lock().unwrap();
            *c += 1;
            return Some(path[path.len() - self.max_correlation_length..].to_vec());
        }
        let mut recent = self.recent_paths.lock().unwrap();
        for existing in recent.iter() {
            if existing.len() == path.len() && existing.iter().eq(path.iter()) {
                let mut c = self.loop_count.lock().unwrap();
                *c += 1;
                return Some(path.to_vec());
            }
            if existing.len() >= 2 && path.len() >= 2 {
                if let Some(cyc) = detect_cycle(existing, path) {
                    let mut c = self.loop_count.lock().unwrap();
                    *c += 1;
                    return Some(cyc);
                }
            }
        }
        recent.push_back(path.to_vec());
        while recent.len() > self.max_recent {
            recent.pop_front();
        }
        None
    }

    pub fn loop_count(&self) -> u64 {
        *self.loop_count.lock().unwrap()
    }

    pub fn reset(&self) {
        self.recent_paths.lock().unwrap().clear();
        *self.loop_count.lock().unwrap() = 0;
    }
}

fn detect_cycle(a: &[String], b: &[String]) -> Option<Vec<String>> {
    let set_a: HashSet<&str> = a.iter().map(|s| s.as_str()).collect();
    let set_b: HashSet<&str> = b.iter().map(|s| s.as_str()).collect();
    let common: Vec<&str> = set_a.intersection(&set_b).copied().collect();
    if common.len() >= 2 {
        Some(common.into_iter().map(|s| s.to_string()).collect())
    } else {
        None
    }
}
