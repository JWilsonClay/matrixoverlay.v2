// src/core/logging/mod.rs
pub mod visual;

use std::fs::{self, OpenOptions};
use std::io::{Write, BufWriter};
use std::path::{Path, PathBuf};
use chrono::Local;

pub use self::visual::*;

pub struct Logger {
    log_dir: PathBuf,
    max_files: usize,
    max_file_size: u64,
}

impl Logger {
    pub fn new(log_dir_str: &str, max_files: usize, max_file_size_mb: u64) -> Self {
        let log_dir = PathBuf::from(log_dir_str);
        if !log_dir.exists() { let _ = fs::create_dir_all(&log_dir); }
        Self { log_dir, max_files, max_file_size: max_file_size_mb * 1024 * 1024 }
    }

    pub fn purge_debug_logs(log_dir: &str) -> std::io::Result<()> {
        let path = Path::new(log_dir);
        if !crate::core::path_utils::is_safe_path(path) { return Ok(()); }
        if path.exists() && path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let p = entry.path();
                if let Ok(m) = fs::symlink_metadata(&p) { if m.file_type().is_symlink() { continue; } }
                if p.is_file() && p.extension().map_or(false, |e| e == "log") {
                    let _ = fs::remove_file(p);
                }
            }
        }
        Ok(())
    }

    pub fn write_to_file(&self, filename: &str, content: &str) {
        let path = self.log_dir.join(filename);
        if !crate::core::path_utils::is_safe_path(&path) { return; }

        if let Ok(m) = fs::metadata(&path) { if m.len() > self.max_file_size { self.rotate_logs(filename); } }

        if let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
            }
            let mut writer = BufWriter::new(file);
            let ts = Local::now().format("%Y-%m-%dT%H:%M:%S");
            let _ = writeln!(writer, "[{}] {}", ts, content);
        }
    }

    fn rotate_logs(&self, filename: &str) {
        for i in (1..self.max_files).rev() {
            let old = self.log_dir.join(format!("{}.{}", filename, i));
            let new = self.log_dir.join(format!("{}.{}", filename, i + 1));
            if old.exists() { let _ = fs::rename(old, new); }
        }
        let curr = self.log_dir.join(filename);
        let first = self.log_dir.join(format!("{}.1", filename));
        let _ = fs::rename(curr, first);
    }
}
