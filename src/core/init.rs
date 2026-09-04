// src/core/init.rs
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::env;
use std::sync::Arc;
use simplelog::{WriteLogger, Config as LogConfig, LevelFilter};
use crate::core::config::Config;
use crate::core::version;
use crate::build_logger;

pub fn init_logging(config: &Config) -> Result<()> {
    version::print_startup_info();
    
    // [GL-2, Phase 8.7] `build_logging_enabled` had no reader anywhere in the
    // tree — a stored, GUI-less, permanently-inert flag. It is wired here rather
    // than deleted because the field already exists in users' `config.json` and
    // `#[serde(deny_unknown_fields)]` (C-02) would reject those files the moment
    // the struct lost it. It now does the obvious thing its name claims: gate
    // the build-event log.
    if env::args().any(|a| a == "debug-build") {
        if config.logging.build_logging_enabled {
            build_logger::log_build_event("cargo build --release", &config.logging.log_path);
        } else {
            eprintln!("build logging is disabled (logging.build_logging_enabled = false)");
        }
        return Ok(());
    }

    if config.logging.enabled {
        let log_dir = Path::new(&config.logging.log_path);
        
        // [HARDENING] Validate path safety before creating directory or file
        if !crate::core::path_utils::is_safe_path(log_dir) {
            log::error!("SECURITY ALERT: Unsafe log path detected in config: {}", config.logging.log_path);
            return Ok(()); // Fail safe: don't crash, but don't log to unsafe path
        }

        if !log_dir.exists() {
            fs::create_dir_all(log_dir).context("Failed to create log directory")?;
        }
        
        let log_file_path = log_dir.join("matrix_overlay.log");
        let _ = WriteLogger::init(
            LevelFilter::Info,
            LogConfig::default(),
            fs::File::create(&log_file_path).context("Failed to create log file")?
        );
        println!("Logging enabled. Directory: {}", config.logging.log_path);
    } else {
        env_logger::init();
    }
    Ok(())
}

pub fn setup_xcb() -> Result<(Arc<xcb::Connection>, i32)> {
    let (conn, screen_num) = xcb::Connection::connect(None).context("Failed to connect to X server. Verify DISPLAY environment variable.")?;
    Ok((Arc::new(conn), screen_num))
}

pub fn setup_autostart() -> Result<()> {
    let home = env::var("HOME").context("HOME not set")?;
    let home_path = Path::new(&home);
    
    // [HARDENING] Validate home path safety
    if !crate::core::path_utils::is_safe_path(home_path) {
        anyhow::bail!("Security Alert: Unsafe HOME path detected.");
    }

    let autostart_dir = home_path.join(".config/autostart");
    if !autostart_dir.exists() { fs::create_dir_all(&autostart_dir)?; }
    
    let desktop_file = autostart_dir.join("matrix-overlay.desktop");

    if desktop_file.exists() {
        let metadata = fs::symlink_metadata(&desktop_file)?;
        if metadata.file_type().is_symlink() {
            log::warn!("SECURITY ALERT: Autostart file is a symlink. Removing for safety.");
            let _ = fs::remove_file(&desktop_file);
        }
    }

    if !desktop_file.exists() {
        let current_exe = env::current_exe()?;
        let content = format!(
            "[Desktop Entry]\nType=Application\nName=Matrix Overlay\nExec=\"{}\"\nX-GNOME-Autostart-enabled=true\n",
            current_exe.to_string_lossy()
        );
        fs::write(&desktop_file, content)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&desktop_file, fs::Permissions::from_mode(0o644));
        }
    }
    Ok(())
}

pub fn safe_exec(cmd: &str, args: &[&str]) -> Result<()> {
    // [HARDENING] Whitelist of allowed execution binaries
    let bin = Path::new(cmd).file_name().and_then(|f| f.to_str()).unwrap_or("");
    if bin != "notify-send" && bin != "xdg-open" && bin != "git" && bin != "xsetroot" {
         log::error!("SECURITY ALERT: safe_exec attempted unauthorized binary: {}", cmd);
         anyhow::bail!("Unauthorized binary execution");
    }

    let status = std::process::Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context(format!("Failed to execute {}", cmd))?;
    
    if !status.success() {
        log::warn!("Command {} failed with status {}", cmd, status);
    }
    Ok(())
}
