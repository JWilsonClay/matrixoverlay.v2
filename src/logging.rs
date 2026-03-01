// src/logging.rs
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{Write, BufWriter};
use std::path::PathBuf;
use chrono::{Local, DateTime};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemState {
    pub id: String,
    pub item_type: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StateCapture {
    pub timestamp: String,
    pub monitor: usize,
    pub items: Vec<ItemState>,
}

pub struct Logger {
    log_dir: PathBuf,
    max_files: usize,
    max_file_size: u64,
}

impl Logger {
    pub fn new(log_dir: &str, max_files: usize, max_file_size_mb: u64) -> Self {
        let log_dir = PathBuf::from(log_dir);
        if !log_dir.exists() {
            let _ = fs::create_dir_all(&log_dir);
        }
        Self {
            log_dir,
            max_files,
            max_file_size: max_file_size_mb * 1024 * 1024,
        }
    }

    pub fn log_state(&self, capture: &StateCapture) {
        let json = serde_json::to_string(capture).unwrap_or_default();
        self.write_to_file("state.log", &json);
        
        let ascii = self.render_ascii_view(capture);
        self.write_to_file("visual.log", &ascii);
    }

    /// Purges all debug log files in the specified directory.
    pub fn purge_debug_logs(log_dir: &str) -> std::io::Result<()> {
        let path = std::path::Path::new(log_dir);
        if path.exists() && path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    // Only delete files ending in .log
                    if path.extension().map_or(false, |ext| ext == "log") {
                        std::fs::remove_file(path)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn write_to_file(&self, filename: &str, content: &str) {
        let path = self.log_dir.join(filename);
        
        // Rotation check
        if let Ok(metadata) = fs::metadata(&path) {
            if metadata.len() > self.max_file_size {
                self.rotate_logs(filename);
            }
        }

        if let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) {
            let mut writer = BufWriter::new(file);
            let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S");
            let _ = writeln!(writer, "[{}] {}", timestamp, content);
        }
    }

    fn rotate_logs(&self, filename: &str) {
        for i in (1..self.max_files).rev() {
            let old_path = self.log_dir.join(format!("{}.{}", filename, i));
            let new_path = self.log_dir.join(format!("{}.{}", filename, i + 1));
            if old_path.exists() {
                let _ = fs::rename(old_path, new_path);
            }
        }
        let current_path = self.log_dir.join(filename);
        let first_backup = self.log_dir.join(format!("{}.1", filename));
        let _ = fs::rename(current_path, first_backup);
    }

    pub fn purge_old_logs(&self) {
        let now = Local::now();
        if let Ok(entries) = fs::read_dir(&self.log_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        let duration = now.signed_duration_since(DateTime::<Local>::from(modified));
                        if duration.num_hours() > 24 {
                            let _ = fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }

    fn render_ascii_view(&self, capture: &StateCapture) -> String {
        let width = 80;
        let height = 24;
        let mut grid = vec![vec![' '; width]; height];

        // Draw border
        for x in 0..width {
            grid[0][x] = '-';
            grid[height - 1][x] = '-';
        }
        for y in 0..height {
            grid[y][0] = '|';
            grid[y][width - 1] = '|';
        }

        for item in &capture.items {
            let gx = (item.x / 1920.0 * width as f64) as usize;
            let gy = (item.y / 1080.0 * height as f64) as usize;
            
            if gx < width && gy < height {
                let marker = match item.item_type.as_str() {
                    "rain" => ':',
                    "metric" => 'M',
                    _ => '?',
                };
                grid[gy][gx] = marker;
            }
        }

        let mut output = format!("Monitor: {}\n", capture.monitor);
        for row in grid {
            output.push_str(&row.iter().collect::<String>());
            output.push('\n');
        }
        output
    }
}
