use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::io::Read;
use crate::metrics::{MetricId, MetricValue, MetricCollector};
use crate::core::path_utils;

/// Collector for Custom Files (e.g. shared logs).
#[derive(Debug)]
pub struct FileCollector {
    files: Vec<crate::core::config::CustomFile>,
}

impl FileCollector {
    pub fn new(files: Vec<crate::core::config::CustomFile>) -> Self {
        Self { files }
    }
}

impl MetricCollector for FileCollector {
    fn id(&self) -> &'static str { "files" }
    fn label(&self) -> &'static str { "Files" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        for file in &self.files {
            let file_path = Path::new(&file.path);
            if !path_utils::is_safe_path(file_path) {
                log::warn!("Access Denied: Path traversal detected or unsafe area: {}", file.path);
                map.insert(MetricId::Custom(file.metric_id.clone()), MetricValue::String("ACCESS DENIED".to_string()));
                continue;
            }

            let mut content = "N/A".to_string();
            if let Ok(mut f) = fs::File::open(file_path) {
                let mut buffer = Vec::new();
                if f.by_ref().take(64 * 1024).read_to_end(&mut buffer).is_ok() {
                    let s = String::from_utf8_lossy(&buffer);
                    let s = s.trim();
                    if file.tail {
                        content = s.lines().last().unwrap_or("").to_string();
                    } else {
                        content = s.to_string();
                    }
                }
            }
            map.insert(MetricId::Custom(file.metric_id.clone()), MetricValue::String(content));
        }
        map
    }
}
