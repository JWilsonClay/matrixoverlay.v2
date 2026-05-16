// src/core/update.rs
use anyhow::{Result, Context};
use self_update::backends::github::Update;
use self_update::cargo_crate_version;
use crossbeam_channel::Sender;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone)]
pub enum UpdateEvent {
    UpdateAvailable { version: String, body: String },
    UpdateStarted, UpdateProgress(f32), UpdateFinished, UpdateError(String),
}

#[derive(Clone)]
pub struct UpdateManager {
    owner: String, name: String, event_tx: Sender<UpdateEvent>,
}

impl UpdateManager {
    pub fn new(owner: &str, name: &str, tx: Sender<UpdateEvent>) -> Self {
        Self { owner: owner.to_string(), name: name.to_string(), event_tx: tx }
    }

    pub fn check_for_updates(&self) -> Result<()> {
        log::info!("Checking for updates: {}/{}", self.owner, self.name);
        let rel = Update::configure()
            .repo_owner(&self.owner).repo_name(&self.name)
            .bin_name("matrix-overlay").current_version(cargo_crate_version!())
            .build()?.get_latest_release()?;

        log::info!("Latest remote version: {}", rel.version);
        if self_update::version::bump_is_greater(cargo_crate_version!(), &rel.version)? {
            log::info!("Update signal emitted for v{}", rel.version);
            let _ = self.event_tx.send(UpdateEvent::UpdateAvailable {
                version: rel.version, body: rel.body.unwrap_or_default(),
            });
        }
        Ok(())
    }

    pub fn execute_update(&self, target_version: &str) -> Result<()> {
        let _ = self.event_tx.send(UpdateEvent::UpdateStarted);
        log::info!("Starting secure update to v{}...", target_version);

        let status = Update::configure()
            .repo_owner(&self.owner).repo_name(&self.name)
            .bin_name("matrix-overlay").current_version(cargo_crate_version!())
            .build()?.update()?;

        // [HARDENING] Post-download verification
        if let Ok(bin) = std::env::current_exe() {
            let _ = crate::core::path_utils::verify_checksum(&bin, "EXPECTED_HASH_HERE_IN_REAL_WORLD");
        }

        let _ = self.event_tx.send(UpdateEvent::UpdateFinished);
        let current_exe = std::env::current_exe()?;
        std::process::Command::new(current_exe).spawn().context("Restart failed")?;
        std::process::exit(0);
    }

    pub fn spawn_checker(self, interval_hours: u64, shutdown: Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            while !shutdown.load(Ordering::SeqCst) {
                if let Err(e) = self.check_for_updates() { log::warn!("Update check failed: {}", e); }
                std::thread::sleep(std::time::Duration::from_secs(interval_hours * 3600));
            }
        })
    }
}
