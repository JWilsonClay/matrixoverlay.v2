// src/version.rs
use std::process::Command;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERSION
}

/// Checks for other running instances of matrix-overlay or matrix_overlay.
/// Returns a list of PIDs of other instances.
pub fn detect_other_instances() -> Vec<u32> {
    let current_pid = std::process::id();
    let mut pids = Vec::new();
    
    // Check both hyphen and underscore variants
    for pattern in &["matrix-overlay", "matrix_overlay"] {
        let output = Command::new("pgrep")
            .arg("-f")
            .arg(pattern)
            .output()
            .ok();

        if let Some(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Ok(pid) = line.parse::<u32>() {
                    if pid != current_pid && !pids.contains(&pid) {
                        pids.push(pid);
                    }
                }
            }
        }
    }
    pids
}

/// Kills other running instances of matrix-overlay.
pub fn kill_other_instances() {
    let others = detect_other_instances();
    if !others.is_empty() {
        println!("Killing {} existing instance(s)...", others.len());
        for pid in others {
            let _ = Command::new("kill").arg("-9").arg(pid.to_string()).status();
        }
    }
}

pub fn print_startup_info() {
    println!("Matrix Overlay v{} (PID: {})", VERSION, std::process::id());
    kill_other_instances();
}
