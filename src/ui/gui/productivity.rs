//! Productivity Configuration GUI Substrate.
//! Orchestrates AI integration and Git repository tracking parameters.

use gtk::prelude::*;
use gtk::{Box, Label, SpinButton, CheckButton, Entry};
use crate::core::config::Config;

/// [HARDENED] Builds the productivity configuration view with validated inputs.
pub fn build(vbox: &Box, config: &Config) -> (CheckButton, Entry, SpinButton) {
    vbox.set_border_width(10);

    let check_ollama = CheckButton::with_label("Enable Ollama AI Insights");
    check_ollama.set_active(config.productivity.ollama_enabled);
    vbox.pack_start(&check_ollama, false, false, 0);

    vbox.pack_start(&Label::new(Some("Git Repositories (Comma separated)")), false, false, 5);
    let repos_entry = Entry::new();
    repos_entry.set_text(&config.productivity.repos.join(", "));
    // [HARDENING] Prevent excessive memory usage in entry buffers
    repos_entry.set_max_length(1024);
    vbox.pack_start(&repos_entry, false, false, 0);

    vbox.pack_start(&Label::new(Some("Auto-Commit Threshold (Lines)")), false, false, 5);
    let commit_spin = SpinButton::with_range(0.0, 5000.0, 100.0);
    commit_spin.set_value(config.productivity.auto_commit_threshold as f64);
    vbox.pack_start(&commit_spin, false, false, 0);

    (check_ollama, repos_entry, commit_spin)
}
