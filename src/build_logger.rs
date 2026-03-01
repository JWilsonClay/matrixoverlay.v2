// src/build_logger.rs
use std::process::Command;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use chrono::Local;

pub fn log_build_event(cmd: &str, log_dir: &str) {
    let log_dir = PathBuf::from(log_dir);
    if !log_dir.exists() {
        let _ = fs::create_dir_all(&log_dir);
    }
    let log_path = log_dir.join("build.log");

    println!("Executing build command: {}", cmd);
    
    let output = Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .output();

    let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S");
    let mut log_content = format!("\n--- Build Event: {} ---\nCommand: {}\n", timestamp, cmd);

    match output {
        Ok(out) => {
            let status = if out.status.success() { "SUCCESS" } else { "FAILURE" };
            log_content.push_str(&format!("Status: {}\n", status));
            log_content.push_str("STDOUT:\n");
            log_content.push_str(&String::from_utf8_lossy(&out.stdout));
            log_content.push_str("\nSTDERR:\n");
            log_content.push_str(&String::from_utf8_lossy(&out.stderr));
        }
        Err(e) => {
            log_content.push_str(&format!("Error executing command: {}\n", e));
        }
    }

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = write!(file, "{}", log_content);
    }
}
