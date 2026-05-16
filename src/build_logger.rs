// src/build_logger.rs
use std::process::Command;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use chrono::Local;

/// [HARDENED] Logs a build event with path validation and shell-less execution.
/// Loop 4: Added binary whitelisting to prevent arbitrary execution.
pub fn log_build_event(cmd: &str, log_dir_str: &str) {
    let log_dir = Path::new(log_dir_str);
    
    // [HARDENING] Validate log path safety
    if !crate::core::path_utils::is_safe_path(log_dir) {
        eprintln!("Security Alert: Unsafe build log directory: {}", log_dir_str);
        return;
    }

    if !log_dir.exists() {
        let _ = fs::create_dir_all(log_dir);
    }
    let log_path = log_dir.join("build.log");

    println!("Executing build command: {}", cmd);
    
    // [HARDENING] Avoid bash -c. Use split for safe execution.
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() { return; }

    // [HARDENING] Whitelist of allowed build binaries
    let bin = parts[0];
    if bin != "cargo" && bin != "make" && bin != "npm" {
        eprintln!("Security Alert: Unauthorized build binary: {}", bin);
        return;
    }

    let output = Command::new(bin)
        .args(&parts[1..])
        .stdin(std::process::Stdio::null())
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

    // [HARDENING] Safe append permissions
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = write!(file, "{}", log_content);
    }
}
