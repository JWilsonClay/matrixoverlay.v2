// src/core/productivity/mod.rs
pub mod ai;

use anyhow::{Result, Context};
use std::path::Path;
use std::fs;
use git2::Repository;
use crate::core::config::Config;
use crate::core::path_utils;

/// Orchestrates the auto-commit cycle for all configured repositories.
pub fn run_auto_commit_cycle(config: &Config) -> Result<()> {
    log::info!("Starting auto-commit cycle for {} repos...", config.productivity.repos.len());
    for repo_path in &config.productivity.repos {
        let path = Path::new(repo_path);
        if !path_utils::is_safe_path(path) { continue; }
        if let Ok(meta) = fs::symlink_metadata(path) {
             if meta.file_type().is_symlink() { continue; }
        }
        if let Ok(repo) = Repository::open(path) {
            if let Err(e) = handle_repo_auto_commit(&repo, config) {
                log::error!("Failed auto-commit in {}: {}", repo_path, e);
            }
        }
    }
    Ok(())
}

fn handle_repo_auto_commit(repo: &Repository, config: &Config) -> Result<()> {
    let mut index = repo.index()?;
    if repo.statuses(None)?.is_empty() { return Ok(()); }

    let mut total_lines = 0;
    if let Ok(diff) = repo.diff_index_to_workdir(None, None) {
        if let Ok(stats) = diff.stats() { total_lines = stats.insertions() + stats.deletions(); }
    }
    if total_lines < config.productivity.auto_commit_threshold as usize { return Ok(()); }

    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let tree = repo.find_tree(index.write_tree()?)?;
    let parent = repo.head()?.peel_to_commit()?;
    let sig = repo.signature()?;

    let message = if config.productivity.ollama_enabled {
        ai::generate_ai_commit_message(repo).unwrap_or_else(|_| "Auto-commit (Matrix Overlay)".to_string())
    } else {
        "Auto-commit (Matrix Overlay)".to_string()
    };

    repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[&parent])?;
    Ok(())
}
