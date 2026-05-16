//! Sovereign File Collector Substrate.
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::io::Read;
use crate::metrics::{MetricId, MetricValue, MetricCollector};
use crate::core::path_utils;

/// [HARDENED] Collector for custom file monitoring with safety boundaries.
#[derive(Debug)]
pub struct FileCollector {
    files: Vec<crate::core::config::CustomFile>,
}

impl FileCollector {
    pub fn new(files: Vec<crate::core::config::CustomFile>) -> Self { Self { files } }
}

impl MetricCollector for FileCollector {
    fn id(&self) -> &'static str { "files" }
    fn label(&self) -> &'static str { "Files" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        for file in &self.files {
            let p = Path::new(&file.path);
            
            // [HARDENING] Strict path and symlink validation
            if !path_utils::is_safe_path(p) { continue; }
            if let Ok(m) = fs::symlink_metadata(p) { if m.file_type().is_symlink() { continue; } }

            let mut content = "N/A".to_string();
            // [HARDENING] Resource-limited read (max 64KB)
            if let Ok(mut f) = fs::File::open(p) {
                let mut buf = Vec::new();
                if f.by_ref().take(65536).read_to_end(&mut buf).is_ok() {
                    let s = String::from_utf8_lossy(&buf);
                    content = if file.tail { s.trim().lines().last().unwrap_or("").to_string() }
                              else { s.trim().to_string() };
                }
            }
            map.insert(MetricId::Custom(file.metric_id.clone()), MetricValue::String(content));
        }
        map
    }
}
