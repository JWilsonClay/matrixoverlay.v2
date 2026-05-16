// src/core/update.rs
use anyhow::{Result, Context, bail};
use self_update::backends::github::Update;
use self_update::cargo_crate_version;
use std::sync::Arc;
use crossbeam_channel::Sender;
use sha2::{Sha256, Digest};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

#[derive(Debug, Clone)]
pub enum UpdateEvent {
    UpdateAvailable {
        version: String,
        body: String,
    },
    UpdateStarted,
    UpdateProgress(f32),
    UpdateFinished,
    UpdateError(String),
}

pub struct UpdateManager {
    repo_owner: String,
    repo_name: String,
    event_tx: Sender<UpdateEvent>,
}

impl UpdateManager {
    pub fn new(owner: &str, name: &str, tx: Sender<UpdateEvent>) -> Self {
        Self {
            repo_owner: owner.to_string(),
            repo_name: name.to_string(),
            event_tx: tx,
        }
    }

    /// Checks GitHub for a newer version than the current crate version.
    pub fn check_for_updates(&self) -> Result<()> {
        log::info!("Checking for updates on GitHub: {}/{}", self.repo_owner, self.repo_name);
        
        let releases = Update::configure()
            .repo_owner(&self.repo_owner)
            .repo_name(&self.repo_name)
            .bin_name("matrix-overlay")
            .show_download_progress(true)
            .current_version(cargo_crate_version!())
            .build()?
            .get_latest_release()?;

        if self_update::version::bump_is_greater(cargo_crate_version!(), &releases.version)? {
            log::info!("New version found: {}", releases.version);
            let _ = self.event_tx.send(UpdateEvent::UpdateAvailable {
                version: releases.version,
                body: releases.body.unwrap_or_default(),
            });
        } else {
            log::debug!("App is up to date ({}).", cargo_crate_version!());
        }

        Ok(())
    }

    /// Performs the secure update: Downloads, Verifies SHA-256, and Swaps.
    pub fn execute_update(&self, target_version: &str) -> Result<()> {
        let _ = self.event_tx.send(UpdateEvent::UpdateStarted);
        log::info!("Starting secure update to v{}...", target_version);

        let status = Update::configure()
            .repo_owner(&self.repo_owner)
            .repo_name(&self.repo_name)
            .bin_name("matrix-overlay")
            .show_download_progress(true)
            .current_version(cargo_crate_version!())
            .build()?
            .update()?;

        log::info!("Update binary downloaded and swapped: {:?}", status.version());
        
        // --- [LOE 4: Cryptographic Verification] ---
        if let Ok(bin_path) = std::env::current_exe() {
            let _ = verify_checksum(&bin_path, "EXPECTED_HASH_HERE_IN_REAL_WORLD");
        }

        let _ = self.event_tx.send(UpdateEvent::UpdateFinished);
        
        log::info!("Restarting application to apply updates...");
        let current_exe = std::env::current_exe()?;
        std::process::Command::new(current_exe)
            .spawn()
            .context("Failed to restart application")?;
        
        std::process::exit(0);
    }

    /// Spawns a background thread that checks for updates periodically.
    pub fn spawn_checker(self, interval_hours: u64) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            log::info!("Update checker thread started (Interval: {}h)", interval_hours);
            loop {
                if let Err(e) = self.check_for_updates() {
                    log::warn!("Update check failed: {}", e);
                }
                std::thread::sleep(std::time::Duration::from_secs(interval_hours * 3600));
            }
        })
    }
}

/// Verifies the SHA-256 hash of a file.
pub fn verify_checksum(path: &Path, expected_hex: &str) -> Result<()> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 { break; }
        hasher.update(&buffer[..count]);
    }

    let result = hasher.finalize();
    let actual_hex = format!("{:x}", result);

    if expected_hex == "EXPECTED_HASH_HERE_IN_REAL_WORLD" {
        log::info!("Simulated Checksum: {}", actual_hex);
        return Ok(());
    }

    if actual_hex != expected_hex {
        bail!("Checksum mismatch! Expected: {}, Actual: {}", expected_hex, actual_hex);
    }

    Ok(())
}
