// src/core/version.rs
use sysinfo::{ProcessExt, System, SystemExt};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str { VERSION }

/// [HARDENED] Detects other instances using sysinfo for precision.
pub fn detect_other_instances() -> Vec<u32> {
    let mut sys = System::new_all();
    sys.refresh_processes();
    let current_pid = sysinfo::get_current_pid().ok();
    
    sys.processes()
        .iter()
        .filter(|(pid, proc)| {
            let name = proc.name();
            (name == "matrix-overlay" || name == "matrix_overlay") 
            && Some(**pid) != current_pid
        })
        .map(|(pid, _)| format!("{}", pid).parse::<u32>().unwrap_or(0))
        .collect()
}

/// [HARDENED] Kills other instances safely.
pub fn kill_other_instances() {
    let others = detect_other_instances();
    if !others.is_empty() {
        log::info!("Sovereign Governance: Purging {} stale instances.", others.len());
        let mut sys = System::new_all();
        sys.refresh_processes();
        for pid_u32 in others {
            if let Some(proc) = sys.process(sysinfo::Pid::from(pid_u32 as usize)) {
                proc.kill();
            }
        }
    }
}

pub fn print_startup_info() {
    log::info!("Matrix Overlay v{} (PID: {})", VERSION, std::process::id());
    kill_other_instances();
}
