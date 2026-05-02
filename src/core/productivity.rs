use anyhow::{bail, Result};
use std::path::Path;
use git2::Repository;
use crate::core::config::Config;
use crate::core::path_utils;

/// Orchestrates the auto-commit cycle for all configured repositories.
pub fn run_auto_commit_cycle(config: &Config) -> Result<()> {
    log::info!("Starting auto-commit cycle for {} repos...", config.productivity.repos.len());
    
    for repo_path in &config.productivity.repos {
        let path = Path::new(repo_path);
        if !path_utils::is_safe_path(path) {
            log::warn!("Skipping unsafe repo path: {}", repo_path);
            continue;
        }

        match Repository::open(path) {
            Ok(repo) => {
                if let Err(e) = handle_repo_auto_commit(&repo, config) {
                    log::error!("Failed to auto-commit in {}: {}", repo_path, e);
                }
            }
            Err(e) => log::warn!("Could not open repo at {}: {}", repo_path, e),
        }
    }
    
    Ok(())
}

fn handle_repo_auto_commit(repo: &Repository, config: &Config) -> Result<()> {
    let mut index = repo.index()?;
    let statuses = repo.statuses(None)?;
    
    if statuses.is_empty() {
        return Ok(());
    }

    // Check line count threshold
    let mut total_diff_lines = 0;
    if let Ok(diff) = repo.diff_index_to_workdir(None, None) {
        if let Ok(stats) = diff.stats() {
            total_diff_lines = stats.insertions() + stats.deletions();
        }
    }

    if total_diff_lines < config.productivity.auto_commit_threshold as usize {
        log::debug!("Skipping auto-commit: {} lines < {} threshold", total_diff_lines, config.productivity.auto_commit_threshold);
        return Ok(());
    }

    // Stage all changes
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let parent_commit = repo.head()?.peel_to_commit()?;
    let sig = repo.signature()?;

    let message = if config.productivity.ollama_enabled {
        generate_ai_commit_message(repo).unwrap_or_else(|_| "Auto-commit (Matrix Overlay)".to_string())
    } else {
        "Auto-commit (Matrix Overlay)".to_string()
    };

    repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[&parent_commit])?;
    log::info!("Auto-committed to {}: {}", repo.path().display(), message);

    Ok(())
}

fn generate_ai_commit_message(repo: &Repository) -> Result<String> {
    // Basic diff for Ollama
    let diff = repo.diff_index_to_workdir(None, None)?;
    let mut diff_text = Vec::new();
    diff.print(git2::DiffFormat::Patch, |_, _, line| {
        diff_text.extend_from_slice(line.content());
        true
    })?;

    let diff_str = String::from_utf8_lossy(&diff_text);
    let truncated_diff = if diff_str.len() > 4000 {
        format!("{}... [truncated]", &diff_str[..4000])
    } else {
        diff_str.to_string()
    };

    let prompt = format!(
        "Generate a concise one-line git commit message for the following diff:\n\n{}",
        truncated_diff
    );

    let client = reqwest::blocking::Client::new();
    let body = serde_json::json!({
        "model": "qwen2.5-coder:7b-instruct-q5_K_M",
        "prompt": prompt,
        "stream": false
    });

    let res = client.post("http://localhost:11434/api/generate")
        .json(&body)
        .send()?
        .json::<serde_json::Value>()?;

    if let Some(msg) = res["response"].as_str() {
        Ok(msg.trim().trim_matches('"').to_string())
    } else {
        bail!("Failed to get message from Ollama")
    }
}
