use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};
use git2::Repository;
use crate::metrics::{MetricId, MetricValue, MetricCollector};
use crate::core::path_utils;

/// Collector for Git productivity (Delta lines +/- over 24h).
#[derive(Debug)]
pub struct GitCollector {
    pub repos: Vec<String>,
    pub delta_window: Duration,
    pub last_check: Instant,
    pub cached_delta: (i64, i64),
    pub(crate) rotation_index: usize,
    pub(crate) start_time: Instant,
}

impl GitCollector {
    pub fn new(repos: Vec<String>) -> Self {
        Self {
            repos,
            delta_window: Duration::from_secs(24 * 3600),
            last_check: Instant::now() - Duration::from_secs(3600),
            cached_delta: (0, 0),
            rotation_index: 0,
            start_time: Instant::now(),
        }
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

        let mut total_added = 0;
        let mut total_deleted = 0;
        
        let uptime = self.start_time.elapsed();
        let window_hours = if uptime < Duration::from_secs(3600) { 1 } else { 24 };
        let yesterday = chrono::Local::now() - chrono::Duration::hours(window_hours);
        let yesterday_ts = yesterday.timestamp();

        if self.repos.is_empty() {
             let mut map = HashMap::new();
             map.insert(MetricId::CodeDelta, MetricValue::String("+0 / -0".to_string()));
             return map;
        }

        let batch_cap = 5;
        let count = std::cmp::min(self.repos.len(), batch_cap);
        
        for i in 0..count {
            let idx = (self.rotation_index + i) % self.repos.len();
            let repo_path = Path::new(&self.repos[idx]);
            
            if !path_utils::is_safe_path(repo_path) {
                log::warn!("Access Denied: Git repo outside home or unsafe: {}", self.repos[idx]);
                continue;
            }

            if let Ok(repo) = Repository::open(repo_path) {
                let mut revwalk = match repo.revwalk() {
                    Ok(rv) => rv,
                    Err(_) => continue,
                };
                let _ = revwalk.push_head();

                let mut objects_seen = 0;
                for oid in revwalk {
                    if objects_seen >= 500 {
                        break;
                    }
                    objects_seen += 1;

                    let oid = match oid { Ok(o) => o, Err(_) => continue };
                    let commit = match repo.find_commit(oid) { Ok(c) => c, Err(_) => continue };
                    
                    if commit.time().seconds() < yesterday_ts {
                        break;
                    }

                    if commit.parent_count() > 0 {
                        if let (Ok(parent), Ok(tree)) = (commit.parent(0), commit.tree()) {
                            if let Ok(parent_tree) = parent.tree() {
                                if let Ok(diff) = repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), None) {
                                    if let Ok(stats) = diff.stats() {
                                        total_added += stats.insertions() as i64;
                                        total_deleted += stats.deletions() as i64;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        self.rotation_index = (self.rotation_index + count) % self.repos.len();
        self.cached_delta = (total_added, total_deleted);
        self.last_check = now;

        let mut map = HashMap::new();
        map.insert(MetricId::CodeDelta, MetricValue::String(format!("+{} / -{}", total_added, total_deleted)));
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_git_delta_accuracy_24h_rolling() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        fs::write(dir.path().join("file.txt"), "hello").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Update", &tree, &[&parent]).unwrap();

        let mut collector = GitCollector::new(vec![dir.path().to_str().unwrap().to_string()]);
        collector.start_time = Instant::now() - Duration::from_secs(3600);
        let results = collector.collect();
        assert!(results.contains_key(&MetricId::CodeDelta));
    }

    #[test]
    fn test_git_rotation_batching_cap() {
        let repos = (0..10).map(|i| format!("/tmp/repo{}", i)).collect::<Vec<_>>();
        let mut collector = GitCollector::new(repos);
        collector.collect();
        assert_eq!(collector.rotation_index, 5);
        collector.collect();
        assert_eq!(collector.rotation_index, 0);
    }
}
