//! Git productivity metrics collection substrate.
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};
use git2::Repository;
use crate::metrics::{MetricId, MetricValue, MetricCollector};
use crate::core::path_utils;

/// [HARDENED] Collector for Git productivity (Delta lines +/- over 24h).
#[derive(Debug)]
pub struct GitCollector {
    pub repos: Vec<String>,
    pub last_check: Instant,
    pub cached_delta: (i64, i64),
    pub rotation_index: usize,
    pub start_time: Instant,
}

impl GitCollector {
    pub fn new(repos: Vec<String>) -> Self {
        Self {
            repos,
            last_check: Instant::now() - Duration::from_secs(3601),
            cached_delta: (0, 0),
            rotation_index: 0,
            start_time: Instant::now(),
        }
    }

    fn calculate_repo_delta(&self, path: &Path, yesterday_ts: i64) -> (i64, i64) {
        let (mut add, mut del) = (0, 0);
        if let Ok(repo) = Repository::open(path) {
            if let Ok(mut walk) = repo.revwalk() {
                let _ = walk.push_head();
                for (seen, oid) in walk.enumerate() {
                    if seen >= 500 { break; }
                    let oid = match oid { Ok(o) => o, Err(_) => continue };
                    let commit = match repo.find_commit(oid) { Ok(c) => c, Err(_) => continue };
                    if commit.time().seconds() < yesterday_ts { break; }
                    if commit.parent_count() > 0 {
                        if let (Ok(p), Ok(t), Ok(pt)) = (commit.parent(0), commit.tree(), commit.parent(0).and_then(|p| p.tree())) {
                            if let Ok(diff) = repo.diff_tree_to_tree(Some(&pt), Some(&t), None) {
                                if let Ok(stats) = diff.stats() {
                                    add += stats.insertions() as i64;
                                    del += stats.deletions() as i64;
                                }
                            }
                        }
                    }
                }
            }
        }
        (add, del)
    }
}

impl MetricCollector for GitCollector {
    fn id(&self) -> &'static str { "git_delta" }
    fn label(&self) -> &'static str { "Productivity" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let now = Instant::now();
        if now.duration_since(self.last_check) < Duration::from_secs(3600) && self.cached_delta != (0, 0) {
             let mut map = HashMap::new();
             map.insert(MetricId::CodeDelta, MetricValue::String(format!("+{} / -{}", self.cached_delta.0, self.cached_delta.1)));
             return map;
        }

        let mut t_add = 0; let mut t_del = 0;
        let window = if self.start_time.elapsed() < Duration::from_secs(3600) { 1 } else { 24 };
        let y_ts = (chrono::Local::now() - chrono::Duration::hours(window)).timestamp();

        let count = std::cmp::min(self.repos.len(), 5);
        for i in 0..count {
            let idx = (self.rotation_index + i) % self.repos.len();
            let p = Path::new(&self.repos[idx]);
            if path_utils::is_safe_path(p) {
                let (a, d) = self.calculate_repo_delta(p, y_ts);
                t_add += a; t_del += d;
            }
        }
        
        self.rotation_index = (self.rotation_index + count) % self.repos.len();
        self.cached_delta = (t_add, t_del);
        self.last_check = now;

        let mut map = HashMap::new();
        map.insert(MetricId::CodeDelta, MetricValue::String(format!("+{} / -{}", t_add, t_del)));
        map
    }
}
